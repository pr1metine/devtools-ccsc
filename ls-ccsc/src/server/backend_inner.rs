use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use regex::{Captures, Regex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

use crate::server::mplab_project_config::MPLABProjectConfig;
use crate::server::text_document_type::TextDocumentType;
use crate::{Url, utils};

#[derive(Default)]
pub struct BackendInner {
    root_path: Option<PathBuf>,
    mcp: Option<MPLABProjectConfig>,
    docs: HashMap<PathBuf, TextDocumentType>,
}

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
        r#"^(?:>>>|\*\*\*|---)\s+([a-zA-Z]+)\s+(\d+)\s+"([^"\n]*)"\s+Line\s+(\d+)\((\d+),(\d+)\): (.*)$"#,
    ).unwrap();
}

impl BackendInner {
    pub fn set_root_path(&mut self, root_path: PathBuf) {
        self.root_path = Some(root_path);
    }

    #[allow(dead_code)]
    pub fn get_root_path(&self) -> Result<&PathBuf> {
        self.root_path
            .as_ref()
            .ok_or(utils::create_server_error(4, "No root path set".to_owned()))
    }

    pub fn set_mcp(&mut self, mplab: MPLABProjectConfig) {
        self.mcp = Some(mplab);
    }

    #[allow(dead_code)]
    pub fn get_mcp(&self) -> Result<&MPLABProjectConfig> {
        self.mcp.as_ref().ok_or(utils::create_server_error(
            4,
            "No mplab project config set".to_owned(),
        ))
    }

    pub fn insert_docs(&mut self, docs: HashMap<PathBuf, TextDocumentType>) {
        self.docs.extend(docs);
    }

    pub fn get_doc_or_ignored(&mut self, path: PathBuf) -> &mut TextDocumentType {
        self.docs.entry(path).or_insert(TextDocumentType::Ignored)
    }

    pub fn get_doc(&self, path: &PathBuf) -> Result<&TextDocumentType> {
        self.docs.get(path).ok_or(utils::create_server_error(
            4,
            format!("No document found for path: {}", path.display()),
        ))
    }

    pub fn clear(&mut self) {
        self.root_path = None;
        self.docs.clear();
        self.mcp = None;
    }

    pub fn insert_compiler_diagnostics(&mut self, p: Vec<PathBuf>) -> HashMap<Url, Vec<Diagnostic>> {
        type UriDiagnosticMap = HashMap<String, Vec<Diagnostic>>;
        fn get_diagnostics_from_err_paths<P: AsRef<Path>>(paths: Vec<P>) -> UriDiagnosticMap {
            type In1 = (String, i32, String, u32, u32, u32, String);
            type In2 = (String, Diagnostic);
            fn read_to_string(mut file: File) -> Option<String> {
                let mut contents = String::new();
                file.read_to_string(&mut contents).ok().map(|_| contents)
            }
            fn get_captures_from_match(captures: Captures) -> std::result::Result<In1, ()> {
                fn get_capture_as_str<'a>(
                    idx: usize,
                    captures: &'a Captures,
                ) -> std::result::Result<&'a str, ()> {
                    Ok(captures.get(idx).ok_or(())?.as_str())
                }

                let severity = get_capture_as_str(1, &captures)?.to_owned();
                let error_code = get_capture_as_str(2, &captures)?
                    .parse::<i32>()
                    .map_err(|_| ())?;
                let path = get_capture_as_str(3, &captures)?.to_owned();
                let line = get_capture_as_str(4, &captures)?
                    .parse::<u32>()
                    .map_err(|_| ())?;
                let character_start = get_capture_as_str(5, &captures)?
                    .parse::<u32>()
                    .map_err(|_| ())?;
                let character_end = get_capture_as_str(6, &captures)?
                    .parse::<u32>()
                    .map_err(|_| ())?;
                let message = get_capture_as_str(7, &captures)?.to_owned();

                Ok((
                    severity,
                    error_code,
                    path,
                    line,
                    character_start,
                    character_end,
                    message,
                ))
            }
            fn construct_uri_and_diagnostic(input: In1) -> std::result::Result<In2, ()> {
                let (severity, error_code, path, line, character_start, character_end, message) =
                    input;
                let line = line - 1;
                let severity = match severity.as_str() {
                    "Info" => DiagnosticSeverity::Information,
                    "Warning" => DiagnosticSeverity::Warning,
                    "Error" | _ => DiagnosticSeverity::Error,
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

                Ok((path, diagnostic))
            }
            fn has_valid_uri(input: &In2) -> bool {
                let (path, _) = input;
                Path::new(path).exists()
            }
            fn add_diagnostic_to_uri(mut map: UriDiagnosticMap, input: In2) -> UriDiagnosticMap {
                let (uri, diagnostic) = input;
                map.entry(uri).or_insert(vec![]).push(diagnostic);
                map
            }

            paths
                .into_iter()
                .filter_map(|path| File::open(path).ok())
                .filter_map(|file| read_to_string(file))
                .flat_map(|contents| {
                    contents
                        .lines()
                        .filter_map(|line| COMPILER_ERROR_MATCHER.captures(line))
                        .filter_map(|captures| get_captures_from_match(captures).ok())
                        .filter_map(|input| construct_uri_and_diagnostic(input).ok())
                        .filter(has_valid_uri)
                        .collect::<Vec<_>>()
                })
                .fold(HashMap::new(), add_diagnostic_to_uri)
        }

        let diagnostics = get_diagnostics_from_err_paths(p);
        self.docs.iter_mut().for_each(|(_, doc)| {
            match doc {
                TextDocumentType::Ignored => {}
                TextDocumentType::Source(source) => source.get_compiler_diagnostics().clear(),
                //TextDocumentType::MCP(source) => source.get_compiler_diagnostics().clear(),
            }
        });

        let mut out = HashMap::new();
        for (path, diagnostic) in diagnostics {
            match self.get_doc_or_ignored(PathBuf::from(path.clone())) {
                TextDocumentType::Ignored => {}
                TextDocumentType::Source(source) => {
                    source.get_compiler_diagnostics().extend(diagnostic.clone())
                }
                //TextDocumentType::MCP(source) => source.get_compiler_diagnostics().extend(diagnostic),
            }
            out.insert(Url::from_file_path(path).unwrap(), diagnostic);
        }
        out
    }
}
