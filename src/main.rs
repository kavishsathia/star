mod lexer;
mod ast;
mod parser;
mod types;

use parser::Parser;
use types::TypeChecker;

fn main() {
    // Test statement type checking
    let test_stmts = vec![
        // Let bindings
        ("let x: integer = 5;", true),
        ("let y: float = 3.14;", true),
        ("let s: {string} = {\"hello\"};", true),
        ("let b: boolean = true;", true),
        ("let n: integer? = null;", true),
        ("let bad: integer = \"oops\";", false),

        // Const bindings
        ("const PI: float = 3.14;", true),
        ("const BAD: integer = true;", false),

        // If statements
        ("if true { 1 + 2; }", true),
        ("if 1 { 1 + 2; }", false),

        // While statements
        ("while true { 1 + 2; }", true),
        ("while 1 { 1 + 2; }", false),

        // Functions
        ("fn add(a: integer, b: integer): integer { return a + b; }", true),
        ("fn bad(a: integer): integer { return true; }", false),

        // Structs
        ("struct Point { x: integer, y: integer }", true),
    ];

    for (source, should_pass) in test_stmts {
        println!("Testing: {}", source);
        let mut parser = Parser::new(source);
        let stmt = parser.parse_statement();

        let mut checker = TypeChecker::new();
        match checker.check_stmt(&stmt) {
            Ok(()) => {
                if should_pass {
                    println!("  OK\n");
                } else {
                    println!("  UNEXPECTED OK (should have failed)\n");
                }
            }
            Err(e) => {
                if !should_pass {
                    println!("  OK (expected error: {})\n", e.message);
                } else {
                    println!("  UNEXPECTED ERROR: {}\n", e.message);
                }
            }
        }
    }
}

