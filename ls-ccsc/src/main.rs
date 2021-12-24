use std::ops::DerefMut;
use std::path::PathBuf;

use ini::Ini;
use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Point};

use crate::server::{MPLABProjectConfig, TextDocument};
use crate::utils::get_path;

mod server;
mod utils;

#[tower_lsp::async_trait]
impl LanguageServer for server::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        fn get_path_from_option(uri: Option<Url>) -> Result<PathBuf> {
            let uri = uri.ok_or_else(|| Error::new(ErrorCode::InvalidParams))?;

            Ok(utils::get_path(uri)?)
        }
        fn get_mcp_ini(path: &PathBuf) -> Result<Ini> {
            let ini = Ini::load_from_file(utils::find_mcp_file(path)?).map_err(|_| {
                utils::create_server_error(1, "Failed to load MPLAB Project Config".to_owned())
            })?;

            Ok(ini)
        }

        let root_path = get_path_from_option(init.root_uri)?;
        let ini = get_mcp_ini(&root_path)?;
        let config = MPLABProjectConfig::from_ini_to_lsp_result(&ini)?;

        let mut parser = self.get_parser();
        let docs = utils::generate_text_documents(&config, &root_path, parser.deref_mut())?;

        let mut data = self.get_data();
        data.set_root_path(root_path);
        data.set_mcp(config);
        data.insert_docs(docs);

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
        self.info("Server initialized. LSP yet to be fully implemented.".to_owned())
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        fn get_range(node: Node) -> Range {
            let tree_sitter::Range {
                start_point:
                Point {
                    row: start_line,
                    column: start_character,
                },
                end_point:
                Point {
                    row: stop_line,
                    column: stop_character,
                },
                ..
            } = node.range();
            Range {
                start: Position {
                    line: start_line as u32,
                    character: start_character as u32,
                },
                end: Position {
                    line: stop_line as u32,
                    character: stop_character as u32,
                },
            }
        }

        let HoverParams {
            text_document_position_params:
            TextDocumentPositionParams {
                position: Position { line, character },
                text_document: TextDocumentIdentifier { uri },
            },
            ..
        } = params;

        let data = self.get_data();

        let tree = data.get_doc(&get_path(uri)?)?.get_syntax_tree()?;

        let pos = Point {
            row: line as usize,
            column: character as usize,
        };

        let mut cursor = tree.walk();
        while cursor.goto_first_child_for_point(pos).is_some() {}

        let node = cursor.node();

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(node.kind().into())),
            range: Some(get_range(node)),
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
