// expect: 0
// expect: 1
// expect: 2
// expect: 3
// expect: 4
fn main(): integer {
    let i: integer = 0;
    while i < 5 {
        print $i;
        i = i + 1;
    }
    return 0;
}
