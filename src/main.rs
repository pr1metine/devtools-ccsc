use std::fs;
use std::path::Path;

use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;

mod server;

#[tower_lsp::async_trait]
impl LanguageServer for server::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        let root_uri = init.root_uri.ok_or(Error::new(ErrorCode::InvalidRequest))?;

        if root_uri.scheme() != "file" { return Err(Error::new(ErrorCode::InternalError)); }

        self.get_client()
            .log_message(MessageType::Info, "Initializing server...")
            .await;


        {
            let mut log_file = String::from(format!("Root URI: {}", root_uri));
            let stub = Url::parse("file://").unwrap();

            let data_lock = self.get_data();
            let mut data = data_lock.lock().unwrap();

            for c_file_url in fs::read_dir(Path::new(root_uri.path()))
                .map_err(|_| Error::new(ErrorCode::InternalError))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|e| e.as_path().extension().is_some())
                .filter(|e| e.as_path().extension().unwrap() == "c")
                .filter(|e| e.to_str().is_some())
                .filter_map(|e| stub.join(e.to_str().unwrap()).ok())
            {
                log_file.push('\n');
                log_file.push_str(format!("c file found! {}", c_file_url.as_str()).as_str());
                // let c_file_path = Path::new(c_file_url.path());
                // let tree = data.
            }

            data.set_root_uri(root_uri);
        }


        self.get_client()
            .log_message(MessageType::Info, log_file.as_str())
            .await;


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
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.get_client()
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
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| server::Backend::new(client));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
