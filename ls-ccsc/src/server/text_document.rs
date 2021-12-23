use std::path::PathBuf;

use tree_sitter::Tree;

#[derive(Default)]
pub struct TextDocument {
    pub absolute_path: PathBuf,
    pub raw: String,
    pub syntax_tree: Option<Tree>,
    pub is_other: bool,
    pub is_generated: bool,
}