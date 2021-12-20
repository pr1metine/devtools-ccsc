use tree_sitter::{Parser, Point};

fn main() {
    println!("Hello, world!");
    let mut parser = Parser::new();

    parser.set_language(tree_sitter_ccsc::language()).unwrap();

    let input = include_str!("input/ccsc-example.c");
    let tree = parser.parse(input, None).unwrap();
    let mut cursor = tree.walk();

    println!("root: {}", cursor.node().kind());

    while cursor
        .goto_first_child_for_point(Point::new(6, 14))
        .is_some()
    {
        if cursor.node().kind() != "function_definition" {
            continue;
        }

        println!(
            "function name: {}",
            cursor
                .node()
                .child_by_field_name("declarator")
                .unwrap()
                .child_by_field_name("declarator")
                .unwrap()
                .utf8_text(input.as_bytes())
                .unwrap()
        );
    }
}
