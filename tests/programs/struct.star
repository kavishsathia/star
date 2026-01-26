// expect: alice
// expect: 30
struct Person {
    name: string,
    age: integer
}

fn main(): integer {
    let p: Person = new Person {
        name: "alice",
        age: 30
    };
    print p.name;
    print $p.age;
    return 0;
}
