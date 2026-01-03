mod lexer;
mod ast;
mod parser;

use parser::Parser;

fn main() {
    let source = r#"
        fn add(a: int, b: int): int {
            return a + b;
        }

        let x: int = add(1, 2);
        const PI: float = 3.14;

        if x > 0 {
            let y: int! = x * 2;
        } else {
            let y: int = 0;
        }

        for let i: int = 0; i < 10; i = i + 1; {
            print(i);
        }

        struct Point {
            x: int,
            y: int
        }

        let p: Point = new Point { x: 1, y: 2 };

        let val: int = maybeNull??;
        let val2: int = maybeError!!;
        let val3: int = maybeBoth!?!?;

        while x > 0 {
            x = x - 1;
        }
    "#;

    let mut parser = Parser::new(source);
    while !parser.at_end() {
        let stmt = parser.parse_statement();
        println!("{:#?}\n", stmt);
    }
}

