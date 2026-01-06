mod lexer;
mod ast;
mod parser;
mod ir;
mod tast;
mod types;

use parser::Parser;
use types::TypeChecker;

fn main() {
    let source = r#"
        struct Point {
            x: integer,
            y: integer
        }

        error OutOfBounds;

        fn main(): integer {
            let p: Point = new Point { x: 10, y: 20 };
            let xx: integer = p.x;
            print xx;

            fn add(a: integer, b: integer): integer {
                return a + b;
            }

            let x: integer = 0;
            let y: integer = 1;
            let n: integer = 10;

            while n > 0 {
                let tmp: integer = x;
                x = y;
                y = add(tmp, y);
                n = n - 1;
            }

            for let i: integer = 0; i < 10; i = i + 1; {
                print i;
            }

            print x;
            return 0;
        }
    "#;

    println!("Parsing...\n");

    let mut parser = Parser::new(source);
    let program = parser.parse_program();
    println!("AST: {:#?}\n", program);

    println!("Type checking...\n");

    let mut type_checker = TypeChecker::new();
    match type_checker.check_program(&program) {
        Ok(typed_program) => println!("TypedAST: {:#?}", typed_program),
        Err(e) => println!("Type error: {}", e.message),
    }
}
