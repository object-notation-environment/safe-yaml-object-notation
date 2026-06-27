# SYON Semantics

## Value types

| SYON type | Rust type | Notes |
|---|---|---|
| Null | `()` / `Option::None` | Represented as `null` or `~` |
| Boolean | `bool` | `true` / `false` only (no `yes`/`no`/`on`/`off`) |
| Integer | `i64` | Decimal only; no hex/octal/binary literals |
| Float | `f64` | Must have a decimal point; no `Infinity`/`NaN` |
| String | `String` | Bare or double-quoted |
| Sequence | `Vec<Value>` | Ordered, heterogeneous |
| Mapping | `Vec<(String, Value)>` | Ordered; duplicate keys are an error |

## Coercion rules

SYON does **not** perform implicit coercion. The type of a value is determined
solely by its syntactic form:

- `42` is always an integer.
- `42.0` is always a float.
- `"42"` is always a string.
- `true` / `false` are always booleans (case-sensitive).
- `null` / `~` are always null.
- Any other unquoted token is a string.

## Duplicate keys

Duplicate keys in a mapping are a **parse error**. Implementations must reject
documents with duplicate keys rather than silently overwriting earlier values.

## Encoding errors

A SYON file that is not valid UTF-8 must be rejected with a decode error before
any lexing begins.

## Error model

All errors are **fatal**: there is no partial-result or best-effort mode.
Implementations return either a complete AST or an error with:

- the byte offset of the error,
- the line and column (1-based),
- a human-readable message.
