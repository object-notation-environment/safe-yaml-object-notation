# SYON Specification

This directory contains the normative specification for Safe YAML Object Notation (SYON).

| Document | Contents |
|---|---|
| [01-lexer.md](01-lexer.md) | Token types and lexical rules |
| [02-grammar.md](02-grammar.md) | Formal grammar (EBNF) |
| [03-semantics.md](03-semantics.md) | Value types, coercion rules, error model |

## Relationship to YAML

SYON is a strict *safe subset* of YAML 1.2:

- Every valid SYON document is a valid YAML document.
- Not every valid YAML document is a valid SYON document.

Excluded features: anchors (`&`), aliases (`*`), explicit tags (`!!`),
directives (`%YAML`, `%TAG`), multi-document streams (`---`/`...`),
block scalars (`|`, `>`), flow indicators used outside inline context.
