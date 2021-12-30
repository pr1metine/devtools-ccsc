use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use ini::Ini;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tree_sitter::Point;

use server::text_document_type::TextDocumentType;

use crate::server::{Backend, CCSCResponse, MPLABProjectConfig, TextDocument};

mod server;
mod utils;

lazy_static! {
    // 0th capture group: Entire match
    // 1st capture group: Error type
    // 2nd capture group: Error code
    // 3rd capture group: File path
    // 4th capture group: Line number
    // 5th capture group: Character start
    // 6th capture group: Character end
    // 7th capture group: Error message
    static ref COMPILER_ERROR_MATCHER: Regex = Regex::new(
        r#"^>>>\s+([a-zA-Z]+)\s+(\d+)\s+"([^"\n]*)"\s+Line\s+(\d+)\((\d+),(\d+)\): (.*)$"#,
    ).unwrap();
}

#[tower_lsp::async_trait]
impl LanguageServer for server::Backend {
    async fn initialize(&self, init: InitializeParams) -> Result<InitializeResult> {
        fn get_path_from_option(uri: Option<Url>) -> Result<PathBuf> {
            let uri = uri.ok_or_else(|| Error::new(ErrorCode::InvalidParams))?;

            Ok(utils::get_path(&uri)?)
        }
        fn get_mcp_ini(path: &PathBuf) -> Result<Ini> {
            let ini = Ini::load_from_file_noescape(utils::find_mcp_file(path)?).map_err(|_| {
                utils::create_server_error(1, "Failed to load MPLAB Project Config".to_owned())
            })?;

            Ok(ini)
        }

        let root_path = get_path_from_option(init.root_uri)?;
        let ini = get_mcp_ini(&root_path)?;
        let config = MPLABProjectConfig::from_ini_to_lsp_result(&ini)?;

        let docs = TextDocumentType::from_mcp(&config, &root_path, self.get_parser())?;

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
        self.get_data().clear();
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
        fn get_diagnostics_from_err_paths<P: AsRef<Path>>(paths: Vec<P>) -> HashMap<Url, Vec<Diagnostic>> {
            type In1 = (String, i32, String, u32, u32, u32, String);
            type In2 = (Url, Diagnostic);
            type UriDiagnosticMap = HashMap<Url, Vec<Diagnostic>>;
            fn read_to_string(mut file: File) -> Option<String> {
                let mut contents = String::new();
                file.read_to_string(&mut contents).ok().map(|_| contents)
            }
            fn get_captures_from_match(captures: Captures) -> std::result::Result<In1, ()> {
                fn get_capture_as_str<'a>(idx: usize, captures: &'a Captures) -> std::result::Result<&'a str, ()> {
                    Ok(captures.get(idx).ok_or(())?.as_str())
                }

                let severity = get_capture_as_str(1, &captures)?.to_owned();
                let error_code = get_capture_as_str(2, &captures)?.parse::<i32>().map_err(|_| ())?;
                let path = get_capture_as_str(3, &captures)?.to_owned();
                let line = get_capture_as_str(4, &captures)?.parse::<u32>().map_err(|_| ())?;
                let character_start = get_capture_as_str(5, &captures)?.parse::<u32>().map_err(|_| ())?;
                let character_end = get_capture_as_str(6, &captures)?.parse::<u32>().map_err(|_| ())?;
                let message = get_capture_as_str(7, &captures)?.to_owned();

                Ok((severity, error_code, path, line, character_start, character_end, message))
            }
            fn construct_uri_and_diagnostic(input: In1) -> std::result::Result<In2, ()> {
                let (severity, error_code, path, line, character_start, character_end, message) = input;
                let line = line - 1;
                let severity = match severity {
                    _ => DiagnosticSeverity::Error,
                };

                let diagnostic = Diagnostic {
                    severity: Some(severity),
                    code: Some(NumberOrString::Number(error_code)),
                    range: Range {
                        start: Position::new(line, character_start),
                        end: Position::new(line, character_end),
                    },
                    message,
                    source: Some("ccsc-compiler".to_string()),
                    ..Default::default()
                };

                Url::from_file_path(path).map(|uri| (uri, diagnostic))
            }
            fn add_diagnostic_to_uri(mut map: UriDiagnosticMap, input: In2) -> UriDiagnosticMap {
                let (uri, diagnostic) = input;
                map.entry(uri).or_insert(vec![]).push(diagnostic);
                map
            }

            paths.into_iter()
                .filter_map(|path| File::open(path).ok())
                .filter_map(|file| read_to_string(file))
                .flat_map(|contents| {
                    contents
                        .lines()
                        .filter_map(|line| COMPILER_ERROR_MATCHER.captures(line))
                        .filter_map(|captures| get_captures_from_match(captures).ok())
                        .filter_map(|input| construct_uri_and_diagnostic(input).ok())
                        .collect::<Vec<_>>()
                })
                .fold(HashMap::new(), add_diagnostic_to_uri)
        }

        let diagnostics = get_diagnostics_from_err_paths(deconstruct_to_paths(params));

        for (uri, diagnostics) in diagnostics {
            self.handle_response(Ok(CCSCResponse::from_diagnostics(uri, diagnostics)))
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
            let mut data = this.get_data();
            let doc_type = data.get_doc_or_ignored(path);

            let out = match doc_type {
                TextDocumentType::Ignored => CCSCResponse::ignore_file(uri),
                TextDocumentType::Source(doc) => generate_response(uri, doc.get_syntax_errors()?),
                TextDocumentType::MCP(doc) => generate_response(uri, doc.get_syntax_errors()?),
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
                    doc.absolute_path.display(),
                    log
                )];
                let diagnostics = doc.get_syntax_errors()?;
                let out = CCSCResponse::new(Some(logs), Some((result, diagnostics)));
                Ok(out)
            }

            let (uri, changes) = deconstruct_input(params);
            let path = utils::get_path(&uri)?;

            let mut data = this.get_data();
            let doc = data.get_doc_or_ignored(path);
            let out = match doc {
                TextDocumentType::Ignored => CCSCResponse::ignore_file(uri),
                TextDocumentType::Source(doc) => reparse_doc(doc, changes, uri)?,
                TextDocumentType::MCP(doc) => reparse_doc(doc, changes, uri)?,
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
                                doc.included_files
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

        let data = self.get_data();
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
