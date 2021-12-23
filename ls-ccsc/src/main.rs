use std::fs;
use std::fs::File;
use std::io::Read;

use ini::Ini;
use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::jsonrpc::ErrorCode::InternalError;
use tower_lsp::lsp_types::*;
use tree_sitter::Point;

use crate::server::{Backend, MPLABProjectConfig, TextDocument};

mod server;

#[tower_lsp::async_trait]
impl LanguageServer for server::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        let root_uri = init
            .root_uri
            .ok_or_else(|| Error::new(ErrorCode::InvalidParams))?;

        if root_uri.scheme() != "file" {
            return Err(Error::new(ErrorCode::InvalidParams));
        }

        self.get_client()
            .log_message(
                MessageType::Info,
                format!(
                    "Initializing server... Received Root URI '{}'",
                    root_uri.as_str()
                ),
            )
            .await;

        let root_path = root_uri.to_file_path().map_err(|_| {
            Backend::create_server_error(1, "Failed to resolve Root URI".to_owned())
        })?;

        let ini = Ini::load_from_file(
            Backend::find_mcp_file(&root_path).map_err(|_e| Backend::create_server_error(2, _e))?,
        )
            .map_err(|_e| Backend::create_server_error(3, _e.to_string()))?;


        let mut data = self.get_data().lock().unwrap();
        let config = MPLABProjectConfig::from_ini_with_root(&ini, root_path, &mut data.parser)
            .map_err(|_e| Backend::create_server_error(4, _e))?;
        data.mplab = Some(config);

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::Incremental,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let HoverParams {
            text_document_position_params:
                TextDocumentPositionParams {
                    position: Position { line, character },
                    text_document: TextDocumentIdentifier { uri },
                },
            ..
        } = params;

        let data = self.get_data().lock().unwrap();

        let tree = &data
            .mplab
            .as_ref()
            .ok_or(Backend::create_server_error(5, "MPLAB Config has not been loaded...".into()))?
            .files
            .get(
                &uri.to_file_path()
                    .map_err(|_e| Error::new(ErrorCode::InternalError))?,
            )
            .ok_or(Error {
                code: ErrorCode::ServerError(69420),
                message: format!("URI ({}) not found!", uri.as_str()),
                data: None,
            })?
            .syntax_tree
            .as_ref()
            .ok_or_else(|| Error::new(ErrorCode::ServerError(666)))?;

        let row = line as usize;
        let column = character as usize;
        let pos = Point { row, column };

        let mut cursor = tree.walk();
        while cursor.goto_first_child_for_point(pos).is_some() {}

        let node = cursor.node();
        let Point {
            row: start_line,
            column: start_character,
        } = node.range().start_point;
        let Point {
            row: stop_line,
            column: stop_character,
        } = node.range().end_point;
        let (start_line, start_character, stop_line, stop_character) = (
            start_line as u32,
            start_character as u32,
            stop_line as u32,
            stop_character as u32,
        );

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(node.kind().into())),
            range: Some(Range {
                start: Position {
                    line: start_line,
                    character: start_character,
                },
                end: Position {
                    line: stop_line,
                    character: stop_character,
                },
            }),
        }))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(server::Backend::new);
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
