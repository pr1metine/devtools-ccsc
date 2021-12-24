use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::jsonrpc::Result;
use tree_sitter::Parser;

use crate::{MPLABProjectConfig, TextDocument, Url, utils};
use crate::server::{MPLABFile, TextDocumentType};

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

pub fn get_path(uri: &Url) -> Result<PathBuf> {
    let path = uri
        .to_file_path()
        .map_err(|_| utils::create_server_error(1, "Failed to resolve Root URI".to_owned()))?;

    Ok(path)
}

pub fn generate_text_documents(
    mcp: &MPLABProjectConfig,
    root_path: &PathBuf,
    parser: &mut Parser,
) -> Result<HashMap<PathBuf, TextDocumentType>> {
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
    fn deconstruct_path(f: &MPLABFile, root_path: &PathBuf) -> (PathBuf, bool) {
        let MPLABFile {
            path,
            is_generated,
            is_other,
            ..
        } = f;
        (root_path.join(path), *is_generated || *is_other)
    }
    fn insert_raw_string(tup: (PathBuf, bool)) -> Option<(PathBuf, String, bool)> {
        read_string(&tup.0).map(|s| (tup.0, s, tup.1)).ok()
    }
    fn create_text_document_type(
        tup: (PathBuf, String, bool),
        parser: &mut Parser,
    ) -> (PathBuf, TextDocumentType) {
        let (p, raw, to_be_ignored) = tup;
        let td = if !to_be_ignored && is_source_file(&p) {
            let tree = parser.parse(&raw, None);
            TextDocumentType::Source(TextDocument::new(p.clone(), raw, tree))
        } else {
            TextDocumentType::Ignored
        };

        (p, td)
    }

    let out = mcp
        .files
        .values()
        .map(|f| deconstruct_path(f, root_path))
        .filter_map(|tup| insert_raw_string(tup))
        .map(|tup| create_text_document_type(tup, parser))
        .collect::<HashMap<_, _>>();

    Ok(out)
}

pub fn is_source_file(path: &PathBuf) -> bool {
    let extension = path.extension();
    if extension.is_none() {
        return false;
    }

    let extension = extension.unwrap();

    extension == "c" || extension == "cpp" || extension == "h"
}
