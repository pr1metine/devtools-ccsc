use std::path::PathBuf;

use tower_lsp::jsonrpc::Result;
use tree_sitter::Tree;

use crate::utils;

pub enum TextDocumentType {
    Ignored,
    Source(TextDocument),
    MCP(TextDocument), // TODO: MCP is not implemented yet
}

#[derive(Debug, Clone, Default)]
pub struct TextDocument {
    pub absolute_path: PathBuf,
    pub raw: String,
    pub syntax_tree: Option<Tree>,
}

impl TextDocument {
    pub fn new(absolute_path: PathBuf, raw: String, syntax_tree: Option<Tree>) -> TextDocument {
        TextDocument {
            absolute_path,
            raw,
            syntax_tree,
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
}
