use std::sync::{Arc, Mutex, MutexGuard};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::MessageType;
use tower_lsp::Client;
use tree_sitter::Parser;

use crate::server::BackendInner;
use crate::server::CCSCResponse;

pub struct Backend {
    client: Client,
    data: Arc<Mutex<BackendInner>>,
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

    pub async fn error(&self, msg: String) {
        self.get_client()
            .show_message(MessageType::Error, msg)
            .await
    }

    pub async fn handle_response(&self, result: Result<CCSCResponse>) {
        match result {
            Ok(CCSCResponse {
                logs,
                uri_diagnostics,
            }) => {
                if let Some(logs) = logs {
                    for log in logs {
                        self.info(log).await;
                    }
                }

                if let Some((uri, diagnostics)) = uri_diagnostics {
                    self.get_client()
                        .publish_diagnostics(uri, diagnostics, None)
                        .await
                }
            }
            Err(err) => {
                self.error(format!("Error code {}: {}", err.code, err.message))
                    .await
            }
        }
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_data(&self) -> MutexGuard<BackendInner> {
        self.data.lock().unwrap()
    }

    pub fn get_parser(&self) -> Arc<Mutex<Parser>> {
        self.parser.clone()
    }

    #[allow(dead_code)]
    pub fn get_parser_as_guard(&self) -> MutexGuard<Parser> {
        self.parser.lock().unwrap()
    }
}
