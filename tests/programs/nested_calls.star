// expect: 25
fn main(): integer {
    fn add(a: integer, b: integer): integer {
        return a + b;
    }
    fn square(x: integer): integer {
        return x * x;
    }
    print $(square(add(2, 3)));
    return 0;
}
