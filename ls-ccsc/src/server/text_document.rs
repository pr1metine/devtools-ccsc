use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    Position, Range, TextDocumentContentChangeEvent,
};
use tree_sitter::{InputEdit, Parser, Point, Tree};

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

    pub fn get_syntax_tree(&self) -> Result<&Tree> {
        self.syntax_tree.as_ref().ok_or(utils::create_server_error(
            3,
            format!(
                "No syntax tree found for file '{}'",
                self.absolute_path.display()
            ),
        ))
    }

    pub fn point_to_byte(&self, point: &Point) -> usize {
        let Point { row, column } = *point;
        self.column_offsets[row] + column
    }

    pub fn byte_to_point(&self, byte: usize) -> Option<Point> {
        for (i, &offset) in self.column_offsets.iter().enumerate() {
            if byte < offset {
                let row = i - 1;
                let column = byte - self.column_offsets[row];
                return Some(Point { row, column });
            }
        }

        Some(Point {
            row: self.column_offsets.len() - 1,
            column: byte - self.column_offsets[self.column_offsets.len() - 1],
        })
    }

    pub fn edit_and_reparse_with_lsp(&mut self, params: Vec<TextDocumentContentChangeEvent>) {
        for (start_line, end_line, start_character, end_character, changed) in params
            .into_iter()
            .filter_map(|tdcce| {
                let TextDocumentContentChangeEvent { range, text, .. } = tdcce;
                range.map(|range| (range, text))
            })
            .map(|(range, text)| {
                let Range {
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
                } = range;
                (
                    start_line as usize,
                    end_line as usize,
                    start_character as usize,
                    end_character as usize,
                    text,
                )
            })
        {
            let curr_input = self.raw.clone();

            let start_position = Point {
                row: start_line,
                column: start_character,
            };
            let old_end_position = Point {
                row: end_line,
                column: end_character,
            };
            let start_byte = self.point_to_byte(&start_position);
            let old_end_byte = self.point_to_byte(&old_end_position);
            let new_end_byte = start_byte + changed.len() - 1;

            let curr_input = utils::add_change_to_string(curr_input, changed, start_byte..old_end_byte);

            self.column_offsets = utils::get_column_offsets(&curr_input);
            let new_end_position = self.byte_to_point(new_end_byte).unwrap();

            let edit = InputEdit {
                start_byte,
                start_position,
                old_end_byte,
                old_end_position,
                new_end_byte,
                new_end_position,
            };

            self.edit_and_reparse(curr_input, edit);
        }
    }

    pub fn edit_and_reparse(&mut self, new_input: String, edit: InputEdit) {
        let tree = self.syntax_tree.as_mut().unwrap();
        tree.edit(&edit);
        let mut parser = self.parser.lock().unwrap();
        let tree = parser.parse(&new_input, self.syntax_tree.as_ref());
        self.raw = new_input;
        self.syntax_tree = tree;
    }
}
