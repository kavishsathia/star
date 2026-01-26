# Structs

## Definition

Define structs at the top level.

```
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
```

## Access

Use dot notation to access fields.

```
struct Point {
    x: integer,
    y: integer
}

fn main(): integer {
    let p: Point = new Point { x: 5, y: 10 };
    print $(p.x + p.y);
    return 0;
}
```

## Mutation

Fields can be reassigned.

```
struct Counter {
    value: integer
}

fn main(): integer {
    let c: Counter = new Counter { value: 0 };
    c.value = c.value + 1;
    print $c.value;
    return 0;
}
```
