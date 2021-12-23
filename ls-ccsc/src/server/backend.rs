use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::Client;
use tower_lsp::jsonrpc::{Error, ErrorCode};
use tree_sitter::Parser;

use crate::server::backend_data::BackendData;

pub struct Backend {
    client: Client,
    data: Arc<Mutex<BackendData>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_ccsc::language()).unwrap();

        Self {
            client,
            data: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_data(&self) -> &Arc<Mutex<BackendData>> {
        &self.data
    }

    pub fn create_server_error(code: i64, message: String) -> Error {
        let code = ErrorCode::ServerError(code);
        Error {
            code,
            message,
            data: None,
        }
    }

    pub fn find_mcp_file(p: &PathBuf) -> Result<PathBuf, String> {
        Ok(
            p.as_path().read_dir().map_err(|_e| _e.to_string())?
                .filter_map(|f| f.ok())
                .map(|f| f.path())
                .filter(|f| f.is_file())
                .filter(|f| f.extension().is_some())
                .filter(|f| f.extension().unwrap() == "mcp")
                .nth(0).ok_or(format!("No .mcp file found inside '{}'", p.display()))?
        )
    }
}
