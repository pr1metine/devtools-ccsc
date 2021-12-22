use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tower_lsp::Client;
use tower_lsp::lsp_types::Url;
use tree_sitter::{Parser, Tree};
use crate::server::text_document::TextDocument;

pub struct BackendData {
    root_uri: Url,
    pub trees: HashMap<Url, TextDocument>,
    parser: Parser,
}

impl BackendData {
    pub fn set_root_uri(&mut self, root_uri: Url) {
        self.root_uri = root_uri;
    }

    pub fn create_new_tree(&mut self, ) -> Tree {
        self.parser.parse(&uri.path, None).unwrap()
    }
}

impl Default for BackendData {
    fn default() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_ccsc::language()).unwrap();
        Self {
            root_uri: Url::parse("file:///").unwrap(),
            trees: Default::default(),
            parser,
        }
    }
}
