use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Diagnostic, TextDocumentContentChangeEvent};
use tree_sitter::{Parser, Tree};

use crate::docs::text_document_type::TextDocumentTypeTrait;
use crate::docs::TextDocumentSource;

#[derive(Clone)]
pub struct TextDocument {
    pub absolute_path: PathBuf,
    pub source: TextDocumentSource,
    pub syntax_tree: Option<Tree>,
    pub parser: Arc<Mutex<Parser>>,
    pub included_files: HashSet<PathBuf>,
    // TODO: Detect cyclic includes
    pub compiler_diagnostics: Vec<Diagnostic>,
}

impl TextDocumentTypeTrait for TextDocument {
    fn set_source(&mut self, source: TextDocumentSource) {
        self.source = source;
    }

    fn set_syntax_tree(&mut self, syntax_tree: Option<Tree>) {
        self.syntax_tree = syntax_tree;
    }

    fn get_source(&self) -> &TextDocumentSource {
        &self.source
    }

    fn get_syntax_tree(&self) -> Result<&Tree> {
        self.syntax_tree
            .as_ref()
            .ok_or(self.construct_file_not_found_error())
    }

    fn get_absolute_path(&self) -> &PathBuf {
        &self.absolute_path
    }

    fn get_included_files(&self) -> &HashSet<PathBuf> {
        &self.included_files
    }

    fn get_compiler_diagnostics(&self) -> &Vec<Diagnostic> {
        &self.compiler_diagnostics
    }

    fn get_parser(&self) -> Arc<Mutex<Parser>> {
        self.parser.clone()
    }

    fn get_mut_syntax_tree(&mut self) -> Result<&mut Tree> {
        let error = self.construct_file_not_found_error();

        self.syntax_tree.as_mut().ok_or(error)
    }

    fn get_mut_compiler_diagnostics(&mut self) -> &mut Vec<Diagnostic> {
        &mut self.compiler_diagnostics
    }

    fn new(absolute_path: PathBuf, raw: String, parser: Arc<Mutex<Parser>>) -> Self {
        let (absolute_path, source, syntax_tree, parser, included_files, compiler_diagnostics) =
            Self::from_string(absolute_path, raw, parser);
        Self {
            absolute_path,
            source,
            syntax_tree,
            parser,
            included_files,
            compiler_diagnostics,
        }
    }
}
