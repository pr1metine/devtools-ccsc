use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::jsonrpc::Result;

use crate::server::mplab_project_config::MPLABProjectConfig;
use crate::server::text_document_type::TextDocumentType;
use crate::utils;

#[derive(Default)]
pub struct BackendInner {
    root_path: Option<PathBuf>,
    mcp: Option<MPLABProjectConfig>,
    docs: HashMap<PathBuf, TextDocumentType>,
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
}
