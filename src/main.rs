mod lexer;
mod ast;
mod parser;
mod ir;
use parser::Parser;

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
    println!("AST: {:#?}", program);
}
