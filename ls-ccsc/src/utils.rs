use std::collections::HashMap;
use std::io::Read;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::jsonrpc::{Error, ErrorCode};
use tower_lsp::jsonrpc::Result;
use tree_sitter::{Parser, Point};

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
    parser: Arc<Mutex<Parser>>,
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
        parser: Arc<Mutex<Parser>>,
    ) -> (PathBuf, TextDocumentType) {
        let (p, raw, to_be_ignored) = tup;
        let td = if !to_be_ignored && is_source_file(&p) {
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

pub fn is_source_file(path: &PathBuf) -> bool {
    let extension = path.extension();
    if extension.is_none() {
        return false;
    }

    let extension = extension.unwrap();

    extension == "c" || extension == "cpp" || extension == "h"
}

pub fn get_column_offsets(input: &String) -> Vec<usize> {
    input
        .chars()
        .enumerate()
        .filter(|(_, c)| c == &'\n')
        .map(|(i, _)| i)
        .fold(vec![0], |mut acc, i| {
            acc.push(i + 1);
            acc
        })
}

pub fn point_to_byte_from_offsets(point: &Point, offsets: &Vec<usize>) -> usize {
    let Point { row, column } = *point;
    offsets[row] + column
}

pub fn byte_to_point_from_offsets(byte: usize, offsets: &Vec<usize>) -> Point {
    for (i, &offset) in offsets.iter().enumerate() {
        if byte < offset {
            return Point::new(i - 1, byte - offsets[i - 1]);
        }
    }
    Point::new(offsets.len() - 1, byte - offsets[offsets.len() - 1])
}

pub fn apply_change(mut target: String, mut diff: String, range: Range<usize>) -> Result<String> {
    let (start_inclusive, end_exclusive) = (range.start, range.end);
    let replacement_end_exclusive = start_inclusive + diff.len();

    unsafe {
        let input = diff.as_mut_vec();
        let destination = target.as_mut_vec();
        for i in start_inclusive..(end_exclusive).min(replacement_end_exclusive) {
            *destination.get_mut(i).ok_or(utils::create_server_error(
                6,
                format!("Target idx ('{}') out of bounds", i),
            ))? = *input
                .get(i - start_inclusive)
                .ok_or(utils::create_server_error(
                    6,
                    format!("Diff idx ('{}') out of bounds", i - start_inclusive),
                ))?;
        }
    }

    if replacement_end_exclusive < end_exclusive {
        for _ in (replacement_end_exclusive)..(end_exclusive) {
            target.remove(replacement_end_exclusive);
        }
    } else if replacement_end_exclusive > end_exclusive {
        target.insert_str(end_exclusive, &diff[end_exclusive - start_inclusive..]);
    }
    Ok(target)
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
}
