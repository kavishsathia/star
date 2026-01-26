# Introduction

Star is a programming language that compiles to WebAssembly.

## Hello World

```
fn main(): integer {
    print "hello";
    return 0;
}
```

Every program needs a `main` function that returns an `integer`. The `print` statement outputs strings.

## Stringify Operator

The `$` operator converts values to strings.

```
fn main(): integer {
    print $(10 + 5);
    return 0;
}
```

Use parentheses when the expression has operators, since `$` has high binding power.
