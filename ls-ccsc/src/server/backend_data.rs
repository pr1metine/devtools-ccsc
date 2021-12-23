use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::Url;
use tree_sitter::Parser;

use crate::server::mplab_project_config::MPLABProjectConfig;
use crate::server::text_document::TextDocument;

pub struct BackendData {
    root_uri: Url,
    pub mplab: Option<MPLABProjectConfig>,
    pub parser: Parser,
}

impl BackendData {
    pub fn set_root_uri(&mut self, root_uri: Url) {
        self.root_uri = root_uri;
    }
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
