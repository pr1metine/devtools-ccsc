use tower_lsp::lsp_types::Url;
use tree_sitter::Tree;

pub struct TextDocument {
    uri: Url,
    raw: String,
    syntax_tree: Tree,
}