// expect_panic
error Hello;

fn main(): integer {
    fn maybe(): integer?! {
        raise new Hello {};
    }

    print $(maybe()!!??);
    return 0;
}
