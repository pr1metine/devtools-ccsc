use tree_sitter::{Parser, Point};

fn main() {
    println!("Hello, world!");
    let mut parser = Parser::new();

    parser.set_language(tree_sitter_ccsc::language()).unwrap();

    let input = include_str!("input/ccsc-example.c");
    let tree = parser.parse(input, None).unwrap();
    let mut cursor = tree.walk();

    // let new_line_idxs = input
    //     .chars()
    //     .enumerate()
    //     .filter(|(_, c)| *c == '\n')
    //     .map(|(i, _)| i)
    //     .collect::<Vec<usize>>();

    println!("root: {}", cursor.node().kind());

    while cursor
        .goto_first_child_for_point(Point::new(8, 22))
        .is_some()
    {
        println!(
            "child: {} -> {}",
            cursor.node().kind(),
            cursor.field_name().unwrap_or("()")
        );
        println!(
            "{}",
            &input[cursor.node().start_byte()..cursor.node().end_byte()]
        );
    }
}
