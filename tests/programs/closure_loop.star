// expect: 0
// expect: 1
// expect: 2
fn main(): integer {
    let printers: {(:integer)} = {};
    let i: integer = 0;
    while i < 3 {
        let captured: integer = i;
        fn printer(): integer {
            print $captured;
            return 0;
        }
        printers = printers + {printer};
        i = i + 1;
    }

    let j: integer = 0;
    while j < 3 {
        printers[j]();
        j = j + 1;
    }
    return 0;
}
