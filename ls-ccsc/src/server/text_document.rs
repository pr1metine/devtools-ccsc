use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use tree_sitter::{InputEdit, Parser, Point, Tree};

use crate::utils;
use crate::utils::{byte_to_point_from_offsets, point_to_byte_from_offsets};

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
    pub raw: String,
    pub column_offsets: Vec<usize>,
    pub syntax_tree: Option<Tree>,
    pub parser: Arc<Mutex<Parser>>,
}

impl TextDocument {
    pub fn new(absolute_path: PathBuf, raw: String, parser: Arc<Mutex<Parser>>) -> TextDocument {
        let column_offsets = utils::get_column_offsets(&raw);
        let mut parser_lock = parser.lock().unwrap();
        let syntax_tree = parser_lock.parse(&raw, None);
        std::mem::drop(parser_lock);

        TextDocument {
            absolute_path,
            column_offsets,
            raw,
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

    pub fn reparse_with_lsp(&mut self, params: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        type In = (usize, usize, usize, usize, String);
        type ReparsingIn = (String, Vec<usize>, InputEdit);
        fn deconstruct_input(tdcce: TextDocumentContentChangeEvent) -> Option<In> {
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
        fn preprocess_for_reparsing(input: In, raw: String, offsets: &Vec<usize>) -> ReparsingIn {
            let (start_line, end_line, start_character, end_character, changed) = input;
            let start_position = Point {
                row: start_line,
                column: start_character,
            };
            let old_end_position = Point {
                row: end_line,
                column: end_character,
            };
            let start_byte = point_to_byte_from_offsets(&start_position, offsets);
            let old_end_byte = point_to_byte_from_offsets(&old_end_position, offsets);
            let new_end_byte = start_byte + changed.len() - 1;

            let curr_input =
                utils::apply_change_to_string(raw, changed, start_byte..old_end_byte);

            let column_offsets = utils::get_column_offsets(&curr_input);
            let new_end_position = byte_to_point_from_offsets(new_end_byte, &column_offsets);

            let edit = InputEdit {
                start_byte,
                start_position,
                old_end_byte,
                old_end_position,
                new_end_byte,
                new_end_position,
            };

            (curr_input, column_offsets, edit)
        }

        let edits = params
            .into_iter()
            .filter_map(|param| deconstruct_input(param))
            .map(|tup| preprocess_for_reparsing(tup, self.raw.clone(), &self.column_offsets))
            .collect::<Vec<ReparsingIn>>();

        for (curr_input, column_offsets, edit) in edits {
            self.reparse(curr_input, column_offsets, edit)?;
        }

        Ok(())
    }

    pub fn reparse(&mut self, input: String, offsets: Vec<usize>, edit: InputEdit) -> Result<()> {
        let tree = self.get_mut_syntax_tree()?;
        tree.edit(&edit);
        let mut parser = self.parser.lock().unwrap();
        let tree = parser.parse(&input, self.syntax_tree.as_ref());

        self.column_offsets = offsets;
        self.raw = input;
        self.syntax_tree = tree;
        Ok(())
    }
}
