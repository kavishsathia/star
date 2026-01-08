mod lexer;
mod ast;
mod aast;
mod parser;
mod ir;
mod tast;
mod types;
mod locals;

use std::time::Instant;
use parser::Parser;
use types::TypeChecker;
use locals::LocalsIndexer;

fn main() {
    let source = r#"
        fn main(): integer {
            let b: integer = 2;

            fn add(a: integer): integer {
                return a + b;
            }
        }
    "#;

    println!("Parsing...\n");

    let parse_start = Instant::now();
    let mut parser = Parser::new(source);
    let program = parser.parse_program();
    let parse_duration = parse_start.elapsed();
    println!("AST: {:#?}\n", program);
    println!("Parsing took: {:?}\n", parse_duration);

    println!("Type checking...\n");

    let typecheck_start = Instant::now();
    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_program(&program);
    let typecheck_duration = typecheck_start.elapsed();
    match result {
        Ok(typed_program) => {
            println!("TypedAST: {:#?}\n", typed_program);
            println!("Type checking took: {:?}\n", typecheck_duration);

            println!("Analyzing...\n");

            let analyze_start = Instant::now();
            let mut indexer = LocalsIndexer::new();
            let analyzed_program = indexer.analyze_program(&typed_program);
            let analyze_duration = analyze_start.elapsed();
            println!("AnalyzedAST: {:#?}\n", analyzed_program);
            println!("Analysis took: {:?}", analyze_duration);
        }
        Err(e) => {
            println!("Type error: {}", e.message);
            println!("Type checking took: {:?}", typecheck_duration);
        }
    }
}
