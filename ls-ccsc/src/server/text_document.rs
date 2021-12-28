use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Diagnostic, Position, Range, TextDocumentContentChangeEvent};
use tree_sitter::{InputEdit, Node, Parser, Point, Query, QueryCursor, Tree, TreeCursor};

use crate::{MPLABProjectConfig, utils};
use crate::server::{MPLABFile, TextDocumentSource};

#[derive(Clone)]
pub enum TextDocumentType {
    Ignored,
    Source(TextDocument),
    #[allow(dead_code)]
    MCP(TextDocument), // TODO: MCP is not implemented yet
}

impl TextDocumentType {
    pub fn from_mcp(
        mcp: &MPLABProjectConfig,
        root_path: &PathBuf,
        parser: Arc<Mutex<Parser>>,
    ) -> Result<HashMap<PathBuf, TextDocumentType>> {
        fn read_string(path: &PathBuf) -> Result<String> {
            let mut file = std::fs::File::open(path).map_err(|e| {
                utils::create_server_error(
                    6,
                    format!(
                        "Could not open file '{}' ('{}')",
                        path.display(),
                        e.to_string()
                    ),
                )
            })?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).map_err(|e| {
                utils::create_server_error(
                    6,
                    format!(
                        "Could not read file '{}' ('{}')",
                        path.display(),
                        e.to_string()
                    ),
                )
            })?;
            Ok(contents)
        }
        fn deconstruct_path(f: &MPLABFile, root_path: &PathBuf) -> (PathBuf, bool) {
            let MPLABFile {
                path,
                is_generated,
                is_other,
                ..
            } = f;
            (root_path.join(path), *is_generated || *is_other)
        }
        fn insert_raw_string(tup: (PathBuf, bool)) -> Option<(PathBuf, String, bool)> {
            read_string(&tup.0).map(|s| (tup.0, s, tup.1)).ok()
        }
        fn create_text_document_type(
            tup: (PathBuf, String, bool),
            parser: Arc<Mutex<Parser>>,
        ) -> (PathBuf, TextDocumentType) {
            let (p, raw, to_be_ignored) = tup;
            let td = if !to_be_ignored && utils::is_source_file(&p) {
                TextDocumentType::Source(TextDocument::new(p.clone(), raw, parser.clone()))
            } else {
                TextDocumentType::Ignored
            };

            (p, td)
        }

        let out = mcp
            .files
            .values()
            .map(|f| deconstruct_path(f, root_path))
            .filter_map(|tup| insert_raw_string(tup))
            .map(|tup| create_text_document_type(tup, parser.clone()))
            .collect::<HashMap<_, _>>();

        Ok(out)
    }
}

#[derive(Clone)]
pub struct TextDocument {
    pub absolute_path: PathBuf,
    pub source: TextDocumentSource,
    pub syntax_tree: Option<Tree>,
    pub parser: Arc<Mutex<Parser>>,
    pub included_files: HashSet<PathBuf>, // TODO: Detect cyclic includes
}

type TDCCE = TextDocumentContentChangeEvent;
impl TextDocument {
    pub fn new(absolute_path: PathBuf, raw: String, parser: Arc<Mutex<Parser>>) -> TextDocument {
        fn get_included_files(
            root_node: Node,
            source: &[u8],
            root_path: &PathBuf,
        ) -> HashSet<PathBuf> {
            // TODO: Cache these queries
            let query = Query::new(
                tree_sitter_ccsc::language(),
                "(preproc_include path: (_) @path) @include",
            )
                .unwrap();
            let include_idx = query.capture_index_for_name("include").unwrap();
            let path_idx = query.capture_index_for_name("path").unwrap();
            let mut query_cursor = QueryCursor::new();

            let out = query_cursor
                .matches(&query, root_node, source)
                .filter(|m| {
                    !m.nodes_for_capture_index(include_idx)
                        .any(|c| c.has_error())
                })
                .filter_map(|m| m.nodes_for_capture_index(path_idx).next())
                .filter_map(|c| c.utf8_text(source).ok())
                .filter(|path_str| path_str.len() > 2)
                .map(|path_str| root_path.join(&path_str[1..path_str.len() - 1]))
                .collect::<HashSet<_>>();

            out
        }

        let source = TextDocumentSource::from(raw);

        let mut parser_lock = parser.lock().unwrap();
        let syntax_tree = parser_lock.parse(source.get_raw(), None);
        std::mem::drop(parser_lock);

        let included_files = get_included_files(
            syntax_tree.as_ref().unwrap().root_node(),
            source.get_raw().as_bytes(),
            &absolute_path,
        );

        TextDocument {
            absolute_path,
            source,
            syntax_tree,
            parser,
            included_files,
        }
    }

    pub fn get_mut_syntax_tree(&mut self) -> Result<&mut Tree> {
        self.syntax_tree.as_mut().ok_or(utils::create_server_error(
            3,
            format!(
                "No syntax tree found for file '{}'",
                self.absolute_path.display()
            ),
        ))
    }

    pub fn get_syntax_tree(&self) -> Result<&Tree> {
        self.syntax_tree.as_ref().ok_or(utils::create_server_error(
            3,
            format!(
                "No syntax tree found for file '{}'",
                self.absolute_path.display()
            ),
        ))
    }

    pub fn reparse_with_lsp(&mut self, params: Vec<TDCCE>) -> Result<String> {
        type In1 = (usize, usize, usize, usize, String);
        type In2 = (Point, Point, String);
        type In3 = (TextDocumentSource, InputEdit);
        type In4 = (TextDocumentSource, Option<Tree>);
        fn deconstruct_input(tdcce: TextDocumentContentChangeEvent) -> Option<In1> {
            let TextDocumentContentChangeEvent { range, text, .. } = tdcce;

            range.map(
                |Range {
                     start:
                         Position {
                             line: start_line,
                             character: start_character,
                         },
                     end:
                         Position {
                             line: end_line,
                             character: end_character,
                         },
                 }| {
                    (
                        start_line as usize,
                        end_line as usize,
                        start_character as usize,
                        end_character as usize,
                        text,
                    )
                },
            )
        }
        fn construct_points_and_change(input: In1) -> In2 {
            let (start_line, end_line, start_character, end_character, changed) = input;
            let start_position = Point::new(start_line as usize, start_character as usize);
            let old_end_position = Point::new(end_line as usize, end_character as usize);
            (start_position, old_end_position, changed)
        }
        fn preprocess_for_reparsing(input: In2, source: TextDocumentSource) -> Result<In3> {
            let (start_position, old_end_position, changed) = input;
            let start_byte = source.get_offset_for_point(&start_position)?;
            let old_end_byte = source.get_offset_for_point(&old_end_position)?;
            let new_end_byte = start_byte + changed.len();

            let curr_input = utils::apply_change(
                source.get_raw().to_owned(),
                changed,
                start_byte..old_end_byte,
            )?;

            let source = TextDocumentSource::from(curr_input);
            let new_end_position = source.get_point_from_byte_idx(new_end_byte)?;

            let edit = InputEdit {
                start_byte,
                start_position,
                old_end_byte,
                old_end_position,
                new_end_byte,
                new_end_position,
            };

            Ok((source, edit))
        }
        fn reparse_to_tree(input: In3, parser: Arc<Mutex<Parser>>, old_tree: &mut Tree) -> In4 {
            let (source, edit) = input;
            old_tree.edit(&edit);

            let mut parser_lock = parser.lock().unwrap();
            let tree = parser_lock.parse(source.get_raw(), Some(old_tree));

            (source, tree)
        }

        let mut log = String::with_capacity(self.source.get_raw().len());
        for param in params
            .into_iter()
            .filter_map(|param| deconstruct_input(param))
            .map(|input| construct_points_and_change(input))
        {
            let param = preprocess_for_reparsing(param, self.source.clone())?;
            let (source, tree) =
                reparse_to_tree(param, self.parser.clone(), self.get_mut_syntax_tree()?);
            self.source = source;
            self.syntax_tree = tree;

            log.push_str(self.source.get_raw());
            log.push_str("\n\n");
            log.push_str(self.get_syntax_tree()?.root_node().to_sexp().as_str());
            log.push_str("\n\n---\n\n");
        }

        Ok(log)
    }

    pub fn get_syntax_errors(&self) -> Result<Vec<Diagnostic>> {
        fn traverse(mut cursor: TreeCursor, diagnostics: &mut Vec<Diagnostic>, raw: &[u8]) {
            let node = cursor.node();

            if node.is_error() {
                let msg = if node.child_count() == 0 && node.byte_range().len() + 1 > 0 {
                    let unexpected_char = node.utf8_text(raw).unwrap();
                    format!("UNEXPECTED '{}'", unexpected_char)
                } else {
                    node.kind().to_owned()
                };

                diagnostics.push(utils::create_syntax_diagnostic(
                    utils::get_range(&node),
                    msg,
                ));
            };

            if node.is_missing() {
                diagnostics.push(utils::create_syntax_diagnostic(
                    utils::get_range(&node),
                    format!("MISSING {}", node.kind()),
                ));
            }

            cursor.goto_first_child();
            for _ in 0..node.child_count() {
                traverse(cursor.node().walk(), diagnostics, raw);
                cursor.goto_next_sibling();
            }
        }

        let mut diagnostics = Vec::new();
        traverse(
            self.get_syntax_tree()?.walk(),
            &mut diagnostics,
            self.source.get_raw().as_bytes(),
        );
        Ok(diagnostics)
    }
}
