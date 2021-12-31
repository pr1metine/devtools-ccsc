use std::ops::Range;
use std::path::PathBuf;

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position};
use tree_sitter::{Node, Point};

use crate::{Url, utils};

pub fn create_server_error(code: i64, message: String) -> Error {
    let code = ErrorCode::ServerError(code);
    Error {
        code,
        message,
        data: None,
    }
}

pub fn find_path_to_mcp(p: &PathBuf) -> Result<PathBuf> {
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

pub fn is_source_file(path: &PathBuf) -> bool {
    let extension = path.extension();
    if extension.is_none() {
        return false;
    }

    let extension = extension.unwrap();

    extension == "c" || extension == "cpp" || extension == "h"
}

pub fn apply_change(target: String, diff: String, range: Range<usize>) -> Result<String> {
    let (start_inclusive, end_exclusive) = (range.start, range.end);

    let mut out =
        Vec::<u8>::with_capacity(target.len() + diff.len() - (end_exclusive - start_inclusive));
    let target = target.as_bytes();
    let diff = diff.as_bytes();

    for i in 0..start_inclusive {
        out.push(*target.get(i).ok_or(utils::create_server_error(
            6,
            format!("Could not find byte at index {}", i),
        ))?);
    }

    for &c in diff {
        out.push(c);
    }

    for i in end_exclusive..target.len() {
        out.push(*target.get(i).ok_or(utils::create_server_error(
            6,
            format!("Could not find byte at index {} ('{:?}')", i, target),
        ))?);
    }

    let out = String::from_utf8(out).map_err(|e| {
        utils::create_server_error(
            6,
            format!("Could not convert bytes to string ('{}')", e.to_string()),
        )
    })?;

    Ok(out)
}

pub fn get_range(node: &Node) -> tower_lsp::lsp_types::Range {
    let tree_sitter::Range {
        start_point:
            Point {
                row: start_line,
                column: start_character,
            },
        end_point: Point {
            row: stop_line,
            column: stop_character,
        },
        ..
    } = node.range();
    tower_lsp::lsp_types::Range {
        start: Position {
            line: start_line as u32,
            character: start_character as u32,
        },
        end: Position {
            line: stop_line as u32,
            character: stop_character as u32,
        },
    }
}

pub fn create_syntax_diagnostic(range: tower_lsp::lsp_types::Range, msg: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::Error),
        message: format!("Syntax error: '{}'", msg),
        source: Some("tree-sitter-ccsc".to_owned()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_change_1() {
        assert_eq!(
            apply_change(
                "abcdefghijklmnopqrstuvwxyz".to_owned(),
                "abcdefghijklmnopqrstuvwxyz".to_owned(),
                0..1,
            )
            .unwrap(),
            "abcdefghijklmnopqrstuvwxyzbcdefghijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_string_change_empty() {
        assert_eq!(
            apply_change("abcdefghijklmnopqrstuvwxyz".to_owned(), "".to_owned(), 0..1).unwrap(),
            "bcdefghijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_string_change_delete() {
        assert_eq!(
            apply_change("abcdefghijklmnopqrstuvwxyz".to_owned(), "".to_owned(), 0..7).unwrap(),
            "hijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_string_change_expansion() {
        assert_eq!(
            apply_change(
                "abcdefghijklmnopqrstuvwxyz".to_owned(),
                "abcdefghijklmnopqrstuvwxyz".to_owned(),
                0..7,
            )
            .unwrap(),
            "abcdefghijklmnopqrstuvwxyzhijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_string_change_reduction() {
        assert_eq!(
            apply_change(
                "abcdefghijklmnopqrstuvwxyz".to_owned(),
                "defg".to_owned(),
                0..7,
            )
            .unwrap(),
            "defghijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_string_change_without_size_change() {
        assert_eq!(
            apply_change(
                "abcdefghijklmnopqrstuvwxyz".to_owned(),
                "leetcode".to_owned(),
                4..12,
            )
            .unwrap(),
            "abcdleetcodemnopqrstuvwxyz"
        )
    }

    #[test]
    fn test_string_change_with_unicode() {
        assert_eq!(
            apply_change("äääääääääü".to_owned(), "leßtäüde".to_owned(), 0..2,).unwrap(),
            "leßtäüdeääääääääü"
        )
    }

    #[test]
    fn test_string_change_with_unicode_mid_sentence() {
        assert_eq!(
            apply_change("äääääääääü".to_owned(), "leßtäüde".to_owned(), 4..8,).unwrap(),
            "ääleßtäüdeäääääü"
        )
    }
}
