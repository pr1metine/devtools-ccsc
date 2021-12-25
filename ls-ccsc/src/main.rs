use std::path::PathBuf;

use ini::Ini;
use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Point};

use crate::server::{Backend, MPLABProjectConfig, TextDocument, TextDocumentType};

mod server;
mod utils;

#[tower_lsp::async_trait]
impl LanguageServer for server::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        fn get_path_from_option(uri: Option<Url>) -> Result<PathBuf> {
            let uri = uri.ok_or_else(|| Error::new(ErrorCode::InvalidParams))?;

            Ok(utils::get_path(&uri)?)
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

        let parser = self.get_parser();
        let docs = utils::generate_text_documents(&config, &root_path, parser)?;

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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        type DOTDP = DidOpenTextDocumentParams;
        fn did_open_with_result(this: &Backend, params: DOTDP) -> Result<String> {
            let DOTDP {
                text_document: TextDocumentItem { uri, .. },
            } = params;

            let path = utils::get_path(&uri)?;

            let mut data = this.get_data();
            let doc = data.get_doc_or_insert_ignored(path)?;
            Ok(format!("Document opened: {}", doc.absolute_path.display()))
        }

        self.log_result(did_open_with_result(self, params)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        type DCTDP = DidChangeTextDocumentParams;
        type TDCCE = TextDocumentContentChangeEvent;
        fn did_change_with_result(this: &Backend, params: DCTDP) -> Result<String> {
            fn deconstruct_input(params: DidChangeTextDocumentParams) -> (Url, Vec<TDCCE>) {
                let DCTDP {
                    text_document: VersionedTextDocumentIdentifier { uri, .. },
                    content_changes,
                } = params;
                (uri, content_changes)
            }

            let (uri, content_changes) = deconstruct_input(params);
            let path = utils::get_path(&uri)?;

            let mut data = this.get_data();
            let doc = data.get_doc_or_insert_ignored(path.clone())?;
            let log = doc.reparse_with_lsp(content_changes)?;

            Ok(format!(
                "Document '{}' changed:\n{}\n",
                doc.absolute_path.display(),
                log
            ))
        }

        self.log_result(did_change_with_result(self, params)).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        fn deconstruct_input(params: HoverParams) -> (u32, u32, Url) {
            let HoverParams {
                text_document_position_params:
                TextDocumentPositionParams {
                    position: Position { line, character },
                    text_document: TextDocumentIdentifier { uri },
                },
                ..
            } = params;
            (line, character, uri)
        }
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
        fn get_output(pos: Point, doc_type: &TextDocumentType) -> Result<Option<Hover>> {
            let out = match doc_type {
                TextDocumentType::Source(doc) => {
                    let tree = doc.get_syntax_tree()?;
                    let mut cursor = tree.walk();
                    while cursor.goto_first_child_for_point(pos).is_some() {}

                    let node = cursor.node();

                    Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(node.kind().into())),
                        range: Some(get_range(node)),
                    })
                }
                _ => None,
            };
            Ok(out)
        }

        let (line, character, uri) = deconstruct_input(params);

        let data = self.get_data();
        let doc_type = data.get_doc(&utils::get_path(&uri)?)?;
        let pos = Point {
            row: line as usize,
            column: character as usize,
        };

        get_output(pos, doc_type)
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
