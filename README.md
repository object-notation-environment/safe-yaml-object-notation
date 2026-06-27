# SYON — Safe YAML Object Notation

SYON is a YAML-inspired, minimal object-notation language designed for safety and predictability.
It supports the core data model of YAML — scalars, sequences, and mappings — while deliberately
excluding anchors, aliases, arbitrary tags, and multi-document streams.

## Goals

- **Safe**: no executable directives, no reference cycles, no arbitrary type coercion.
- **Readable**: indentation-based, human-friendly syntax.
- **Embeddable**: a single Rust library crate with no unsafe code.

## Workspace layout

```
crates/
  syon-parser/   # tokenizer + winnow-based parser, produces an AST
  syon-cli/      # `syon` binary — parses a .syon file and prints the AST as JSON
spec/            # language specification
```

## Quick start

```bash
task build-parser-crate
task run-cli-binary -- examples/hello.syon
```

## Spec

See [`spec/README.md`](spec/README.md) for the full language specification.

## License

MIT
