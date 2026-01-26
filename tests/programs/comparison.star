// expect: true
// expect: false
// expect: true
// expect: false
// expect: true
// expect: true
fn main(): integer {
    print $(5 < 10);
    print $(10 < 5);
    print $(5 <= 5);
    print $(5 > 10);
    print $(10 > 5);
    print $(5 >= 5);
    return 0;
}
