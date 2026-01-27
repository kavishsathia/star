// expect: 42
fn main(): integer {
    let x: integer = 42;

    fn outer(): (:string) {
        fn inner(): string {
            return $x;
        }
        return inner;
    }

    print outer()();
    return 0;
}
