// expect: 120
fn main(): integer {
    fn factorial(n: integer): integer {
        if n <= 1 {
            return 1;
        }
        return n * factorial(n - 1);
    }

    print $(factorial(5));
    return 0;
}
