use tower_lsp::jsonrpc::{Result, ErrorCode, Error};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _init: InitializeParams) -> Result<InitializeResult> {
        Err(Error::new(ErrorCode::InternalError))
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::Info, "Server initialized. LSP yet to be fully implemented.")
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

    let (service, messages) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
