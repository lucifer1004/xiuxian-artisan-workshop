# Omni AST

> Unified AST Utilities using ast-grep.

## Overview

This crate provides a unified interface for AST-based code analysis across the Omni DevEnv project. Built on top of ast-grep for high-performance pattern matching.

## Features

- Multi-language AST support
- Pattern-based code search
- Syntax tree traversal
- Code transformation support

## Usage

```rust
use omni_ast::AstAnalyzer;

let analyzer = AstAnalyzer::new();
let ast = analyzer.parse("src/main.py")?;
let functions = analyzer.find_functions(&ast)?;
```

## Supported Languages

- Python
- Julia (feature-gated tree-sitter parser)
- Rust
- JavaScript/TypeScript
- Go
- Java

## Testing

- Julia parser snapshots use `xiuxian-testing` `ScenarioFramework` under `tests/fixtures/scenarios/`
- Low-level Julia parser unit snapshots remain in `src/snapshots/`

## License

Apache-2.0
