use tower_lsp::jsonrpc::Result;
use tree_sitter::Point;

use crate::utils;

#[derive(Debug, Clone, PartialEq)]
pub struct TextDocumentSource {
    raw: String,
    positions: Vec<Vec<usize>>,
}

impl TextDocumentSource {
    pub fn get_raw(&self) -> &str {
        &self.raw
    }

    pub fn get_offset_for_point(&self, point: &Point) -> Result<usize> {
        let Point { row, mut column } = *point;
        let row_vec: &Vec<usize> = if row < self.positions.len() {
            self.positions.get(row).ok_or(utils::create_server_error(
                9,
                format!(
                    "Row out of bounds in spite of bound checking ({}, {})",
                    row, column
                ),
            ))?
        } else {
            let out = self.positions.last().ok_or(utils::create_server_error(
                9,
                format!("No rows in content string even though the string is guaranteed to have at least one row at all times ({}, {})", row, column),
            ))?;
            column = usize::MAX;
            out
        };

        let out = if column < row_vec.len() {
            row_vec.get(column).ok_or(utils::create_server_error(
                9,
                format!(
                    "Column out of bounds in spite of bound checking ({}, {})",
                    row, column
                ),
            ))?
        } else {
            row_vec.last().ok_or(utils::create_server_error(
                9,
                format!("No columns in content string even though the string is guaranteed to have at least one character in a row at all times ({}, {})", row, column),
            ))?
        };

        Ok(*out)
    }

    pub fn get_point_from_byte_idx(&self, byte: usize) -> Result<Point> {
        fn get_character_position(positions: &Vec<usize>, byte: usize) -> Result<usize> {
            let out =
                positions
                    .iter()
                    .position(|&x| x == byte)
                    .ok_or(utils::create_server_error(
                        9,
                        format!("Byte out of bounds ({})", byte),
                    ))?;

            Ok(out)
        }

        for (line, &row_pos) in self
            .positions
            .iter()
            .filter_map(|row| row.first())
            .enumerate()
        {
            if byte < row_pos {
                return Ok(Point {
                    row: line - 1,
                    column: get_character_position(&self.positions[line - 1], byte)?,
                });
            }
        }

        Ok(Point {
            row: self.positions.len() - 1,
            column: get_character_position(&self.positions[self.positions.len() - 1], byte)?,
        })
    }
}

impl From<String> for TextDocumentSource {
    fn from(raw: String) -> Self {
        let mut curr_character_offset = 0;
        let mut positions = Vec::<Vec<usize>>::new();
        let mut curr_line = Vec::<usize>::new();

        for c in raw.chars() {
            curr_line.push(curr_character_offset);
            curr_character_offset += c.len_utf8();

            if c == '\n' {
                positions.push(curr_line);
                curr_line = Vec::<usize>::new();
            }
        }

        // Push EOF
        curr_line.push(curr_character_offset);
        positions.push(curr_line);

        Self { raw, positions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_line() {
        let expected = TextDocumentSource {
            raw: "Hello, world!".to_string(),
            positions: vec![vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]],
        };
        let actual = TextDocumentSource::from("Hello, world!".to_string());
        assert_eq!(expected, actual);

        assert_eq!(Point::new(0, 0), actual.get_point_from_byte_idx(0).unwrap());
    }

    #[test]
    fn test_new_line_at_end_of_line() {
        let expected = TextDocumentSource {
            raw: "Hello, world!\n".to_string(),
            positions: vec![vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13], vec![]],
        };
        let actual = TextDocumentSource::from("Hello, world!\n".to_string());
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_multiline() {
        let expected = TextDocumentSource {
            raw: "Hello, world!\nHow are you?\nUghhhh.....\n".to_string(),
            positions: vec![
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                vec![14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26],
                vec![27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38],
                vec![],
            ],
        };
        let actual =
            TextDocumentSource::from("Hello, world!\nHow are you?\nUghhhh.....\n".to_string());
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_prod() {
        let content = TextDocumentSource {
            raw: "\nint add(int a, int b) {\n\treturn a + b;\n}".to_string(),
            positions: vec![
                vec![0],
                vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                    23, 24,
                ],
                vec![25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39],
                vec![40],
            ],
        };
        let actual_content =
            TextDocumentSource::from("\nint add(int a, int b) {\n\treturn a + b;\n}".to_string());

        assert_eq!(content, actual_content);

        let start = Point::new(2, 14);
        let end = Point::new(2, 14);
        let start_inclusive = actual_content.get_offset_for_point(&start).unwrap();
        let end_inclusive = actual_content.get_offset_for_point(&end).unwrap();
        let new_string = utils::apply_change(
            actual_content.get_raw().to_string(),
            "\n    ".into(),
            start_inclusive..end_inclusive,
        )
        .unwrap();
        let new_content = TextDocumentSource::from(new_string);

        let expected = TextDocumentSource {
            raw: "\nint add(int a, int b) {\n\treturn a + b;\n    \n}".to_string(),
            positions: vec![
                vec![0],
                vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                    23, 24,
                ],
                vec![25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39],
                vec![40, 41, 42, 43, 44],
                vec![45],
            ],
        };

        assert_eq!(expected, new_content);

        assert_eq!(
            Point::new(3, 4),
            new_content.get_point_from_byte_idx(44).unwrap(),
        );
    }
}
