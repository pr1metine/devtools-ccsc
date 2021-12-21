use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};

mod lsp_ccs_c;

#[tower_lsp::async_trait]
impl LanguageServer for lsp_ccs_c::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        let root_uri = init.root_uri.ok_or(Error::new(ErrorCode::InvalidRequest))?;

        self.get_client().log_message(
            MessageType::Info,
            format!("Initializing server... root_uri = {}", root_uri).as_str(),
        ).await;

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

    let (service, messages) = LspService::new(|client| lsp_ccs_c::Backend::new(client));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
