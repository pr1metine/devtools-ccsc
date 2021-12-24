use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::jsonrpc::Result;
use tree_sitter::Parser;

use crate::{MPLABProjectConfig, TextDocument, Url, utils};
use crate::server::MPLABFile;

pub fn create_server_error(code: i64, message: String) -> Error {
    let code = ErrorCode::ServerError(code);
    Error {
        code,
        message,
        data: None,
    }
}

pub fn find_mcp_file(p: &PathBuf) -> Result<PathBuf> {
    Ok(p.as_path()
        .read_dir()
        .map_err(|e| utils::create_server_error(4, e.to_string()))?
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|f| f.is_file())
        .filter(|f| f.extension().is_some())
        .filter(|f| f.extension().unwrap() == "mcp")
        .nth(0)
        .ok_or(utils::create_server_error(
            4,
            format!("No .mcp file found inside '{}'", p.display()),
        ))?)
}

pub fn get_path(uri: Url) -> Result<PathBuf> {
    let path = uri
        .to_file_path()
        .map_err(|_| utils::create_server_error(1, "Failed to resolve Root URI".to_owned()))?;

    Ok(path)
}

pub fn generate_text_documents(
    mcp: &MPLABProjectConfig,
    root_path: &PathBuf,
    parser: &mut Parser,
) -> Result<HashMap<PathBuf, TextDocument>> {
    fn read_string(path: &PathBuf) -> Result<String> {
        let mut file = std::fs::File::open(path).map_err(|e| {
            utils::create_server_error(
                6,
                format!(
                    "Could not open file '{}' ('{}')",
                    path.display(),
                    e.to_string()
                ),
            )
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            utils::create_server_error(
                6,
                format!(
                    "Could not read file '{}' ('{}')",
                    path.display(),
                    e.to_string()
                ),
            )
        })?;
        Ok(contents)
    }

    let mut out = HashMap::new();

    for doc in mcp
        .files
        .values()
        .filter(|d| !(d.is_other || d.is_generated))
    {
        let path = root_path.join(&doc.path);

        let extension = path.extension();
        if extension.is_none() {
            continue;
        }

        let extension = extension.unwrap();

        if extension != "c" && extension != "cpp" && extension != "h" {
            continue;
        }

        let raw = read_string(&path)?;

        let tree = parser.parse(&raw, None);
        // TODO: Make key a reference to TextDocument path
        out.insert(path.clone(), TextDocument::new(path, raw, tree));
    }

    Ok(out)
}
