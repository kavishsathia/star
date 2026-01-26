// expect: 42
error Hello;

fn main(): integer {
    fn maybe(): integer?! {
        return 42;
    }

    print $(maybe()!!??);
    return 0;
}
