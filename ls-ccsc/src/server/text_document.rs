use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Diagnostic, Position, Range, TextDocumentContentChangeEvent};
use tree_sitter::{InputEdit, Parser, Point, Tree, TreeCursor};

use crate::server::TextDocumentSource;
use crate::utils;

#[derive(Clone)]
pub enum TextDocumentType {
    Ignored,
    Source(TextDocument),
    #[allow(dead_code)]
    MCP(TextDocument), // TODO: MCP is not implemented yet
}

#[derive(Clone)]
pub struct TextDocument {
    pub absolute_path: PathBuf,
    pub source: TextDocumentSource,
    pub syntax_tree: Option<Tree>,
    pub parser: Arc<Mutex<Parser>>,
}

type TDCCE = TextDocumentContentChangeEvent;
impl TextDocument {
    pub fn new(absolute_path: PathBuf, raw: String, parser: Arc<Mutex<Parser>>) -> TextDocument {
        let source = TextDocumentSource::from(raw);

        let mut parser_lock = parser.lock().unwrap();
        let syntax_tree = parser_lock.parse(source.get_raw(), None);
        std::mem::drop(parser_lock);

        TextDocument {
            absolute_path,
            source,
            syntax_tree,
            parser,
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

        let mut log = String::with_capacity(self.source.get_raw().len());
        for param in params
            .into_iter()
            .filter_map(|param| deconstruct_input(param))
            .map(|input| construct_points_and_change(input))
        {
            let (input, edit) = preprocess_for_reparsing(param, self.source.clone())?;
            let raw = self.reparse(input, edit)?;
            log.push_str(raw);
            log.push_str("\n\n");
            log.push_str(self.get_syntax_tree()?.root_node().to_sexp().as_str());
            log.push_str("\n\n---\n\n");
        }

        Ok(log)
    }

    pub fn reparse(&mut self, content: TextDocumentSource, edit: InputEdit) -> Result<&str> {
        let tree = self.get_mut_syntax_tree()?;
        tree.edit(&edit);
        let mut parser = self.parser.lock().unwrap();
        let tree = parser.parse(content.get_raw(), self.syntax_tree.as_ref());

        self.source = content;
        self.syntax_tree = tree;
        Ok(self.source.get_raw())
    }

    pub fn get_syntax_errors(&self) -> Result<Vec<Diagnostic>> {
        fn convert_ts_range_to_lsp_range(range: tree_sitter::Range) -> Range {
            let tree_sitter::Range {
                start_point:
                    Point {
                        row: start_line,
                        column: start_character,
                    },
                end_point:
                    Point {
                        row: end_line,
                        column: end_character,
                    },
                ..
            } = range;

            Range {
                start: Position {
                    line: start_line as u32,
                    character: start_character as u32,
                },
                end: Position {
                    line: end_line as u32,
                    character: end_character as u32,
                },
            }
        }
        fn traverse(mut cursor: TreeCursor, diagnostics: &mut Vec<Diagnostic>) {
            let node = cursor.node();
            if !node.has_error() {
                return;
            }

            if node.is_error() || node.is_missing() {
                diagnostics.push(Diagnostic::new_simple(
                    convert_ts_range_to_lsp_range(node.range()),
                    node.kind().to_string(),
                ));
            };

            cursor.goto_first_child();
            for _ in 0..node.child_count() {
                traverse(cursor.node().walk(), diagnostics);
                cursor.goto_next_sibling();
            }
        }

        let mut diagnostics = Vec::new();
        traverse(self.get_syntax_tree()?.walk(), &mut diagnostics);
        Ok(diagnostics)
    }
}
