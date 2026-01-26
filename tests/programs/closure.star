// expect: 15
fn main(): integer {
    let x: integer = 10;

    fn add_x(y: integer): integer {
        return x + y;
    }

    print $(add_x(5));
    return 0;
}
