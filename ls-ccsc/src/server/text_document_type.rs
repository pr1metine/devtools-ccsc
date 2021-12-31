use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc;
use tree_sitter::Parser;

use crate::{MPLABProjectConfig, TextDocument, utils};
use crate::server::MPLABFile;

// Replace with Trait?
#[derive(Clone)]
pub enum TextDocumentType {
    Ignored,
    Source(TextDocument),
    #[allow(dead_code)]
    MCP(TextDocument), // TODO: MCP is not implemented yet
}

impl TextDocumentType {
    pub fn index_from_mcp(
        mcp: &MPLABProjectConfig,
        root_path: &PathBuf,
        parser: Arc<Mutex<Parser>>,
    ) -> jsonrpc::Result<HashMap<PathBuf, TextDocumentType>> {
        fn read_string(path: &PathBuf) -> jsonrpc::Result<String> {
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
            parser: Arc<Mutex<Parser>>,
        ) -> (PathBuf, TextDocumentType) {
            let (p, raw, to_be_ignored) = tup;
            let td = if !to_be_ignored && utils::is_source_file(&p) {
                TextDocumentType::Source(TextDocument::new(p.clone(), raw, parser.clone()))
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
            .map(|tup| create_text_document_type(tup, parser.clone()))
            .collect::<HashMap<_, _>>();

        Ok(out)
    }
}
