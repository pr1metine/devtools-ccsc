use tree_sitter::{Parser, Point, Query, QueryCursor};

fn main() {
    println!("Hello, world!");
    let mut parser = Parser::new();

    parser.set_language(tree_sitter_ccsc::language()).unwrap();

    let input = include_str!("input/ccsc-multiple-functions.c");
    let tree = parser.parse(input, None).unwrap();

    // List all functions in input
    let mut cursor = QueryCursor::new();
    let query = Query::new(
        tree_sitter_ccsc::language(),
        "(function_definition 
            type: (_) @function_return_type
            declarator: (function_declarator 
                declarator: (identifier) @function_name
                parameters: (parameter_list) @function_parameters
                )
            body: (compound_statement) @function_body) @function",
    )
    .unwrap();

    println!("Capture names used in query:");
    for capture in query.capture_names().iter() {
        println!("{}", capture);
    }

    let return_type_idx = query
        .capture_index_for_name("function_return_type")
        .unwrap();
    let name_idx = query.capture_index_for_name("function_name").unwrap();
    let parameters_idx = query.capture_index_for_name("function_parameters").unwrap();
    let body_idx = query.capture_index_for_name("function_body").unwrap();
    let function_idx = query.capture_index_for_name("function").unwrap();

    for el in cursor.matches(&query, tree.root_node(), input.as_bytes()) {
        let return_type_node = el.nodes_for_capture_index(return_type_idx).next().unwrap();
        let name_node = el.nodes_for_capture_index(name_idx).next().unwrap();
        let parameters_node = el.nodes_for_capture_index(parameters_idx).next().unwrap();
        let body_node = el.nodes_for_capture_index(body_idx).next().unwrap();
        let function_node = el.nodes_for_capture_index(function_idx).next().unwrap();

        let return_type = return_type_node.utf8_text(input.as_bytes()).unwrap();
        let name = name_node.utf8_text(input.as_bytes()).unwrap();
        let parameters = parameters_node.utf8_text(input.as_bytes()).unwrap();
        let body = body_node.utf8_text(input.as_bytes()).unwrap();
        let function = function_node.utf8_text(input.as_bytes()).unwrap();

        println!("{} {}{}", return_type, name, parameters);
        println!("name: {}", name);
        println!("function return type: {}", return_type);
        println!("function parameters: {}", parameters);
        println!("function body: ```\n{}\n```", body);
        println!("\n```\n{}\n```", function);
    }
}
