use tower_lsp::lsp_types::Url;
use tree_sitter::Parser;

use crate::server::mplab_project_config::MPLABProjectConfig;

pub struct BackendData {
    pub root_uri: Url,
    pub mplab: Option<MPLABProjectConfig>,
    pub parser: Parser,
}

impl Default for BackendData {
    fn default() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_ccsc::language()).unwrap();
        Self {
            root_uri: Url::parse("file:///").unwrap(),
            mplab: None,
            parser,
        }
    }
}
