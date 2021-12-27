use tower_lsp::lsp_types::{Diagnostic, Url};

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

    pub fn from_diagnostics(uri: Url, diagnostics: Vec<Diagnostic>) -> CCSCResponse {
        CCSCResponse::new(None, Some((uri, diagnostics)))
    }

    pub fn from_logs(logs: Vec<String>) -> CCSCResponse {
        CCSCResponse::new(Some(logs), None)
    }
}
