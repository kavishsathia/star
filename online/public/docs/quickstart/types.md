# Types

## Primitives

- `integer` - 64-bit signed integer
- `string` - sequence of characters
- `boolean` - `true` or `false`

## Nullable Types

Append `?` to make a type nullable.

```
fn main(): integer {
    fn maybe(): integer? {
        return 42;
    }
    print $(maybe()??);
    return 0;
}
```

Use `??` to unwrap. Panics if null.

## Error Types

Append `!` to make a type that can hold an error.

```
error MyError;

fn main(): integer {
    fn might_fail(): integer! {
        return 42;
    }
    print $(might_fail()!!);
    return 0;
}
```

Use `!!` to unwrap. Panics if error.

Declare errors at the top level with `error Name;`.

## Combined

Use `?!` for a type that can be null or error. Unwrap with `!!??`.

## Lists

Lists use curly braces.

```
fn main(): integer {
    let nums: {integer} = {1, 2, 3};
    print $(nums[0]);
    return 0;
}
```

Append with `+`:

```
let nums: {integer} = {};
nums = nums + {4};
```

## Function Types

Function types use `{(params): return}`.

```
fn main(): integer {
    let printers: {(:integer)} = {};
    return 0;
}
```
