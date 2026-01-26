# Syntax

## Variables

Declare with `let`. All variables require type annotations.

```
fn main(): integer {
    let x: integer = 42;
    print $x;
    x = 100;
    print $x;
    return 0;
}
```

## If/Else

```
fn main(): integer {
    let x: integer = 10;
    if x > 5 {
        print "big";
    } else {
        print "small";
    }
    return 0;
}
```

## While Loops

Star uses `while` loops.

```
fn main(): integer {
    let i: integer = 0;
    while i < 5 {
        print $i;
        i = i + 1;
    }
    return 0;
}
```

## Operators

Arithmetic: `+`, `-`, `*`, `/`

Comparison: `<`, `>`, `<=`, `>=`, `==`, `!=`

Logical: `and`, `or`, `not`
