// expect_panic
fn main(): integer {
    fn maybe(): integer? {
        return null;
    }

    print $(maybe()??);
    return 0;
}
