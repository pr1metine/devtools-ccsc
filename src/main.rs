use std::fs;
use std::path::Path;

use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;

mod lsp_ccs_c;

#[tower_lsp::async_trait]
impl LanguageServer for lsp_ccs_c::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        let root_uri = init.root_uri.ok_or(Error::new(ErrorCode::InvalidRequest))?;

        if root_uri.scheme() != "file" {
            return Err(Error::new(ErrorCode::InternalError));
        }

        self.get_client()
            .log_message(MessageType::Info, "Initializing server...")
            .await;


        let mut file_log = String::from(format!("Root URI: {}", root_uri));
        for sth in fs::read_dir(Path::new(root_uri.path()))
            .map_err(|_| Error::new(ErrorCode::InternalError))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|e| e.as_path().extension().is_some())
            .filter(|e| e.as_path().extension().unwrap() == "c")
        {
            file_log.push('\n');
            file_log.push_str(format!("c file found! {}", sth.display()).as_str());
        }

        self.get_client()
            .log_message(MessageType::Info, file_log.as_str())
            .await;

        let data_lock = self.get_data();
        let mut data = data_lock.lock().unwrap();
        data.set_root_uri(root_uri);

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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.get_client()
            .log_message(
                MessageType::Info,
                format!("Received did_open for {}", params.text_document.uri).as_str(),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.get_client()
            .log_message(
                MessageType::Info,
                format!("Received did_change for {}", params.text_document.uri).as_str(),
            )
            .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| lsp_ccs_c::Backend::new(client));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
