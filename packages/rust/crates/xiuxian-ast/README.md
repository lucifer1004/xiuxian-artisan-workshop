# xiuxian-ast

> Unified AST Utilities using ast-grep.

## Overview

This crate provides unified AST and structural extraction helpers on top of
ast-grep, plus the Python tree-sitter parser used by the active Rust lanes.

## Features

- Multi-language ast-grep support
- Pattern-based code search
- Syntax tree traversal
- Code transformation support
- Structural semantic fingerprints for supported generic AST languages

## Usage

```rust
use xiuxian_ast::{scan, Lang};

let matches = scan("def hello(): pass", "def $NAME", Lang::Python)?;
```

## Supported Languages

- Python
- Rust
- JavaScript/TypeScript
- Go
- Java

Julia and Modelica no longer live in this crate. The active Wendao lane owns
those languages through `WendaoCodeParser.jl` native routes consumed by
`xiuxian-wendao-julia` over Arrow Flight.

## Testing

- `cargo test -p xiuxian-ast`
- `cargo clippy -p xiuxian-ast --lib --tests -- -D warnings`

## License

Apache-2.0
