<p align="center">
  <img src="assets/logo.png" alt="Star Logo" width="600"/>
</p>

<h1 align="center">Star</h1>

<p align="center">
  <strong>A statically-typed programming language for cross-language library development</strong>
</p>

<p align="center">
  <a href="#installation"><img src="https://img.shields.io/badge/rust-1.70+-orange.svg" alt="Rust Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
  <a href="#"><img src="https://img.shields.io/badge/status-in%20development-yellow.svg" alt="Status"></a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#documentation">Documentation</a> •
  <a href="#contributing">Contributing</a>
</p>

---

## Features

- **Explicit Nullability** — Types are non-nullable by default. Use `T?` for nullable types.
- **First-class Errors** — Error types built into the type system with `T!` for errorable types.
- **Type Safety** — Static type checking catches bugs at compile time.
- **Clean Syntax** — Familiar syntax inspired by TypeScript and Rust.
- **Structs & Functions** — First-class support for user-defined types and higher-order functions.

## Quick Start

```star
fn factorial(n: int): int {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

struct Point {
    x: int,
    y: int
}

let origin: Point = new Point { x: 0, y: 0 };
let result: int = factorial(5);
```

## Type System

```star
let x: int = 42;           // Non-nullable integer
let y: int? = null;        // Nullable integer
let z: int! = getValue();  // Errorable integer
let w: int!? = both();     // Nullable and errorable

// Unwrap operators
let a: int = y??;          // Assert not null
let b: int = z!!;          // Assert not error
let c: int = w!?!?;        // Assert neither
```

## Installation

```bash
git clone https://github.com/kavishsathia/star
cd star
cargo build --release
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
