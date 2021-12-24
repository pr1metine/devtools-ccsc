use std::sync::{Arc, Mutex};

use tower_lsp::Client;
use tower_lsp::lsp_types::MessageType;
use tree_sitter::Parser;

use crate::server::backend_data::BackendData;

pub struct Backend {
    client: Client,
    data: Arc<Mutex<BackendData>>,
    parser: Arc<Mutex<Parser>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_ccsc::language()).unwrap();
        let parser = Arc::new(Mutex::new(parser));
        Self {
            client,
            data: Arc::new(Mutex::new(Default::default())),
            parser,
        }
    }

    pub async fn info(&self, msg: String) {
        self.get_client().log_message(MessageType::Info, msg).await
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_data(&self) -> &Arc<Mutex<BackendData>> {
        &self.data
    }

    pub fn get_parser(&self) -> &Arc<Mutex<Parser>> {
        return &self.parser;
    }
}
