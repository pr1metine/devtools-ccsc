use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::jsonrpc::Result;

use crate::{TextDocument, utils};
use crate::server::mplab_project_config::MPLABProjectConfig;

pub struct BackendData {
    root_path: Option<PathBuf>,
    mcp: Option<MPLABProjectConfig>,
    docs: HashMap<PathBuf, TextDocument>,
}

impl BackendData {
    pub fn set_root_path(&mut self, root_path: PathBuf) {
        self.root_path = Some(root_path);
    }

    pub fn get_root_path(&self) -> Result<&PathBuf> {
        self.root_path
            .as_ref()
            .ok_or(utils::create_server_error(4, "No root path set".to_owned()))
    }

    pub fn set_mcp(&mut self, mplab: MPLABProjectConfig) {
        self.mcp = Some(mplab);
    }

    pub fn get_mcp(&self) -> Result<&MPLABProjectConfig> {
        self.mcp.as_ref().ok_or(utils::create_server_error(
            4,
            "No mplab project config set".to_owned(),
        ))
    }

    pub fn insert_docs(&mut self, docs: HashMap<PathBuf, TextDocument>) {
        docs.into_iter().for_each(|(path, doc)| {
            self.docs.insert(path, doc);
        });
    }

    pub fn get_doc(&self, path: &PathBuf) -> Result<&TextDocument> {
        self.docs.get(path).ok_or(utils::create_server_error(
            4,
            format!("No document found for path: {}", path.display()),
        ))
    }
}

impl Default for BackendData {
    fn default() -> Self {
        Self {
            root_path: None,
            mcp: None,
            docs: HashMap::new(),
        }
    }
}
