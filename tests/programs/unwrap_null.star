// expect: 42
fn main(): integer {
    fn maybe(): integer? {
        return 42;
    }

    print $(maybe()??);
    return 0;
}
