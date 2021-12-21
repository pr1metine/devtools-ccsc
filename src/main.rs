use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tree_sitter::{Language, Parser, Tree, TreeCursor};

use tower_lsp::jsonrpc::{Result, Error, ErrorCode};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use log::{info, error};

struct Data {
    root_uri: Url,
    trees: HashMap<Url, Tree>,
    parser: Parser,
}

impl Default for Data {
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

struct Backend {
    client: Client,
    data: Arc<Mutex<Data>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_ccsc::language()).unwrap();

        Self { 
            client,
            data: Arc::new(Mutex::new(Default::default())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        let root_uri = init.root_uri.ok_or(Error::new(ErrorCode::InvalidRequest))?;

        info!("Initializing server: root_uri = {}", root_uri);

        let mut data = self.data.lock().unwrap();
        data.root_uri = root_uri;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::Incremental,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "lsp-ccs-c".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::Info,
                "Server initialized. LSP yet to be fully implemented.",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
