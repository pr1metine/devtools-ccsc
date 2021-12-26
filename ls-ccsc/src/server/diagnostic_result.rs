use tower_lsp::lsp_types::{Diagnostic, Url};

#[derive(PartialEq, Default)]
pub struct DiagnosticResult {
    pub logs: Option<Vec<String>>,
    pub uri_diagnostics: Option<(Url, Vec<Diagnostic>)>,
}

impl DiagnosticResult {
    pub fn new(
        logs: Option<Vec<String>>,
        uri_diagnostics: Option<(Url, Vec<Diagnostic>)>,
    ) -> Self {
        DiagnosticResult {
            logs,
            uri_diagnostics,
        }
    }

    pub fn from_diagnostics(uri: Url, diagnostics: Vec<Diagnostic>) -> DiagnosticResult {
        DiagnosticResult::new(None, Some((uri, diagnostics)))
    }

    pub fn from_logs(logs: Vec<String>) -> DiagnosticResult {
        DiagnosticResult::new(Some(logs), None)
    }
}
