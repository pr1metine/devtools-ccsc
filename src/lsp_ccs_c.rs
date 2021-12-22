use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tower_lsp::Client;
use tower_lsp::lsp_types::Url;
use tree_sitter::{Parser, Tree};

pub struct BackendData {
    root_uri: Url,
    pub trees: HashMap<Url, Tree>,
    parser: Parser,
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
            trees: Default::default(),
            parser,
        }
    }
}

pub struct Backend {
    client: Client,
    data: Arc<Mutex<BackendData>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_ccsc::language()).unwrap();

        Self {
            client,
            data: Arc::new(Mutex::new(Default::default())),
        }
    }


    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_data(&self) -> &Arc<Mutex<BackendData>> {
        &self.data
    }
}
