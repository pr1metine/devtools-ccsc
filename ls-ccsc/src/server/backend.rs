use std::sync::{Arc, Mutex};

use tower_lsp::Client;
use tree_sitter::Parser;

use crate::server::backend_data::BackendData;

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
