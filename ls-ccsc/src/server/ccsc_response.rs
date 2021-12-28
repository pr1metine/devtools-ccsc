use tower_lsp::lsp_types::{Diagnostic, Url};

use crate::DiagnosticSeverity;

/// Contains logs and / or diagnostics to be sent back to the client
#[derive(PartialEq, Default)]
pub struct CCSCResponse {
    pub logs: Option<Vec<String>>,
    pub uri_diagnostics: Option<(Url, Vec<Diagnostic>)>,
}

impl CCSCResponse {
    pub fn new(logs: Option<Vec<String>>, uri_diagnostics: Option<(Url, Vec<Diagnostic>)>) -> Self {
        CCSCResponse {
            logs,
            uri_diagnostics,
        }
    }

    pub fn from_diagnostics(uri: Url, diagnostics: Vec<Diagnostic>) -> Self {
        CCSCResponse::new(None, Some((uri, diagnostics)))
    }

    #[allow(dead_code)]
    pub fn from_logs(logs: Vec<String>) -> Self {
        CCSCResponse::new(Some(logs), None)
    }

    pub fn ignore_file(uri: Url) -> Self {
        CCSCResponse::from_diagnostics(
            uri,
            vec![Diagnostic::new(
                tower_lsp::lsp_types::Range::default(),
                Some(DiagnosticSeverity::Warning),
                None,
                Some(String::from("ls-ccsc")),
                "Document is ignored".to_string(),
                None,
                None,
            )],
        )
    }
}
