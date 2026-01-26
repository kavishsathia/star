# Functions

## Definition

```
fn add(a: integer, b: integer): integer {
    return a + b;
}

fn main(): integer {
    print $(add(3, 4));
    return 0;
}
```

## Nested Functions

Functions can be nested inside other functions.

```
fn main(): integer {
    fn factorial(n: integer): integer {
        if n <= 1 {
            return 1;
        }
        return n * factorial(n - 1);
    }
    print $(factorial(5));
    return 0;
}
```

## Closures

Nested functions capture variables from their enclosing scope.

```
fn main(): integer {
    let x: integer = 10;
    fn add_x(y: integer): integer {
        return x + y;
    }
    print $(add_x(5));
    return 0;
}
```

## First-Class Functions

Functions can be stored in lists and passed around.

```
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
```
