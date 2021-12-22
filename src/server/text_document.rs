use std::path::PathBuf;

use tree_sitter::Tree;

pub struct TextDocument {
    pub path: PathBuf,
    pub raw: String,
    pub syntax_tree: Tree,
}
