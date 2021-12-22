use std::fs;
use std::fs::File;
use std::io::Read;

use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::jsonrpc::ErrorCode::InternalError;
use tower_lsp::lsp_types::*;
use tree_sitter::{Point, TreeCursor};

use crate::request::{
    GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
    GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
};
use crate::server::TextDocument;

mod server;

#[tower_lsp::async_trait]
impl LanguageServer for server::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        let root_uri = init.root_uri.ok_or(Error::new(ErrorCode::InvalidParams))?;

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

        let mut log_file = String::from(format!("Root URI: {}", root_uri));

        {
            let data_lock = self.get_data();
            let mut data = data_lock.lock().unwrap();

            for c_path in fs::read_dir(
                root_uri
                    .to_file_path()
                    .map_err(|_e| Error::new(ErrorCode::ParseError))?
                    .as_path(),
            )
                .map_err(|e| Error {
                    code: ErrorCode::ServerError(69),
                    message: e.to_string(),
                    data: None,
                })?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|e| e.as_path().extension().is_some())
                .filter(|e| e.as_path().extension().unwrap() == "c")
                .filter(|e| e.to_str().is_some())
            {
                let mut raw = String::new();
                let mut file =
                    File::open(c_path.as_path()).map_err(|_e| Error::new(InternalError))?;
                file.read_to_string(&mut raw)
                    .map_err(|_e| Error::new(InternalError))?;
                let syntax_tree = data
                    .parser
                    .parse(raw.as_bytes(), None)
                    .ok_or(Error::new(InternalError))?;

                log_file.push('\n');
                log_file.push_str(c_path.to_str().unwrap());

                data.trees.insert(
                    c_path.clone(),
                    TextDocument {
                        path: c_path,
                        raw,
                        syntax_tree,
                    },
                );
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let HoverParams {
            text_document_position_params:
            TextDocumentPositionParams {
                position: Position { line, character },
                text_document: TextDocumentIdentifier { uri },
            },
            ..
        } = params;

        let mut data = self.get_data().lock().unwrap();

        let tree = &data
            .trees
            .get(&uri.to_file_path().map_err(|_e| Error::new(ErrorCode::InternalError))?)
            .ok_or(Error {
                code: ErrorCode::ServerError(69420),
                message: format!("URI ({}) not found!", uri.as_str()).to_string(),
                data: None,
            })?
            .syntax_tree;
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

    let (service, messages) = LspService::new(|client| server::Backend::new(client));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
