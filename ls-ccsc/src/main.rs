use std::path::PathBuf;

use ini::Ini;
use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::Point;

use crate::ccsc_response::CCSCResponse;
use crate::docs::{TextDocument, TextDocumentType};
use crate::docs::text_document_type::TextDocumentTypeTrait;
use crate::mplab_project_config::MPLABProjectConfig;
use crate::server::Backend;

mod ccsc_response;
mod docs;
mod mplab_project_config;
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
            let ini =
                Ini::load_from_file_noescape(utils::find_path_to_mcp(path)?).map_err(|_| {
                    utils::create_server_error(1, "Failed to load MPLAB Project Config".to_owned())
                })?;

            Ok(ini)
        }

        let root_path = get_path_from_option(init.root_uri)?;
        let ini = get_mcp_ini(&root_path)?;
        let config = MPLABProjectConfig::from_ini_to_lsp_result(&ini)?;

        let docs = TextDocumentType::index_from_mcp(&config, &root_path, self.get_parser())?;
        let err_paths = utils::find_paths_to_errs(&root_path)?;

        let diagnostics = {
            let mut data = self.get_inner();
            data.set_root_path(root_path);
            data.set_mcp(config);
            data.insert_docs(docs);
            data.insert_compiler_diagnostics(err_paths)
        };

        for (uri, diagnostic) in diagnostics {
            self.handle_response(Ok(CCSCResponse::from_diagnostics(uri, diagnostic)))
                .await;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::Incremental,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "ls-ccsc".to_string(),
                version: Some("0.2.0-alpha".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let watch = DidChangeWatchedFilesRegistrationOptions {
            watchers: vec![FileSystemWatcher {
                glob_pattern: "**/*.err".to_string(),
                kind: None,
            }],
        };

        self.get_client()
            .register_capability(vec![Registration {
                id: "ccsc/watcher".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: serde_json::to_value(watch).ok(),
            }])
            .await
            .unwrap();
    }

    async fn shutdown(&self) -> Result<()> {
        self.get_inner().clear();
        Ok(())
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        fn deconstruct_to_paths(params: DidChangeWatchedFilesParams) -> Vec<PathBuf> {
            let DidChangeWatchedFilesParams { changes } = params;

            changes
                .into_iter()
                .filter_map(|change| change.uri.to_file_path().ok())
                .collect()
        }

        let mut err_paths = deconstruct_to_paths(params);
        err_paths.sort();
        err_paths.dedup();
        let diagnostics = {
            let mut inner = self.get_inner();
            inner.insert_compiler_diagnostics(err_paths)
        };

        for (uri, diagnostic) in diagnostics {
            self.handle_response(Ok(CCSCResponse::from_diagnostics(uri, diagnostic)))
                .await;
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        type DOTDP = DidOpenTextDocumentParams;
        fn did_open_with_result(this: &Backend, params: DOTDP) -> Result<CCSCResponse> {
            let DOTDP {
                text_document: TextDocumentItem { uri, .. },
            } = params;

            let path = utils::get_path(&uri)?;
            let mut data = this.get_inner();
            let doc_type = data.get_doc_or_ignored(path);

            let out = match doc_type {
                TextDocumentType::Ignored => CCSCResponse::ignore_file(uri),
                TextDocumentType::Source(doc) => generate_response(uri, doc.get_diagnostics()?),
                //TextDocumentType::MCP(doc) => generate_response(uri, doc.get_syntax_errors()?),
            };

            Ok(out)
        }
        fn generate_response(uri: Url, diagnostics: Vec<Diagnostic>) -> CCSCResponse {
            CCSCResponse::new(
                Some(vec![format!("Document opened: {}", uri.as_str())]),
                Some((uri, diagnostics)),
            )
        }

        self.handle_response(did_open_with_result(self, params))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        type DCTDP = DidChangeTextDocumentParams;
        type TDCCE = TextDocumentContentChangeEvent;
        fn did_change_with_result(this: &Backend, params: DCTDP) -> Result<CCSCResponse> {
            fn deconstruct_input(params: DidChangeTextDocumentParams) -> (Url, Vec<TDCCE>) {
                let DCTDP {
                    text_document: VersionedTextDocumentIdentifier { uri, .. },
                    content_changes,
                } = params;
                (uri, content_changes)
            }
            fn reparse_doc(
                doc: &mut TextDocument,
                changes: Vec<TDCCE>,
                result: Url,
            ) -> Result<CCSCResponse> {
                let log = doc.reparse_with_lsp(changes)?;
                let logs = vec![format!(
                    "Document '{}' changed:\n{}\n",
                    doc.get_absolute_path().display(),
                    log
                )];
                let diagnostics = doc.get_diagnostics()?;
                let out = CCSCResponse::new(Some(logs), Some((result, diagnostics)));
                Ok(out)
            }

            let (uri, changes) = deconstruct_input(params);
            let path = utils::get_path(&uri)?;

            let mut data = this.get_inner();
            let doc = data.get_doc_or_ignored(path);
            let out = match doc {
                TextDocumentType::Ignored => CCSCResponse::ignore_file(uri),
                TextDocumentType::Source(doc) => reparse_doc(doc, changes, uri)?,
                //TextDocumentType::MCP(doc) => reparse_doc(doc, changes, uri)?,
            };
            Ok(out)
        }

        self.handle_response(did_change_with_result(self, params))
            .await;
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
        fn get_hover_information(pos: Point, doc_type: &TextDocumentType) -> Result<Option<Hover>> {
            let out = match doc_type {
                TextDocumentType::Source(doc) => {
                    let tree = doc.get_syntax_tree()?;
                    let mut cursor = tree.walk();
                    let mut hover_out = String::new();

                    hover_out.push_str(cursor.node().kind());
                    while cursor.goto_first_child_for_point(pos).is_some() {
                        hover_out.push_str(" > ");
                        hover_out.push_str(cursor.node().kind());
                    }

                    Some(Hover {
                        contents: HoverContents::Array(vec![
                            MarkedString::String(hover_out),
                            MarkedString::String(
                                doc.get_included_files()
                                    .iter()
                                    .filter_map(|s| s.to_str().map(|s2| String::from(s2)))
                                    .reduce(|acc, x| format!("{}\n{}", acc, x))
                                    .unwrap_or("".to_string()),
                            ),
                        ]),
                        range: Some(utils::get_range(&cursor.node())),
                    })
                }
                _ => None,
            };
            Ok(out)
        }

        let (line, character, uri) = deconstruct_input(params);

        let data = self.get_inner();
        let doc_type = data.get_doc(&utils::get_path(&uri)?)?;
        let pos = Point::new(line as usize, character as usize);

        get_hover_information(pos, doc_type)
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
