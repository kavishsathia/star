// expect: yes
// expect: no
fn main(): integer {
    if true {
        print "yes";
    } else {
        print "no";
    }
    if false {
        print "yes";
    } else {
        print "no";
    }
    return 0;
}
