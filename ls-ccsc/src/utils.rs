use std::path::PathBuf;

use tower_lsp::jsonrpc::{Error, ErrorCode};

pub fn create_server_error(code: i64, message: String) -> Error {
    let code = ErrorCode::ServerError(code);
    Error {
        code,
        message,
        data: None,
    }
}

pub fn find_mcp_file(p: &PathBuf) -> Result<PathBuf, String> {
    Ok(p.as_path()
        .read_dir()
        .map_err(|_e| _e.to_string())?
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|f| f.is_file())
        .filter(|f| f.extension().is_some())
        .filter(|f| f.extension().unwrap() == "mcp")
        .nth(0)
        .ok_or(format!("No .mcp file found inside '{}'", p.display()))?)
}
