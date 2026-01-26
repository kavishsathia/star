// expect: 5
// expect: 10
fn main(): integer {
    let a: integer = 3;
    let b: integer = 2;
    print $(a + b);
    print $((a + b) * 2);
    return 0;
}
