// expect_panic
error Hello;

fn main(): integer {
    fn maybe(): integer?! {
        return null;
    }

    print $(maybe()!!??);
    return 0;
}
