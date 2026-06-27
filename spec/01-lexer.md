# SYON Lexer Specification

## Character set

SYON source files are UTF-8 encoded.

## Whitespace

- **Indentation** uses spaces only — tabs are illegal.
- **Line endings** are `\n` (LF) or `\r\n` (CRLF); both are normalised to `\n`.
- Trailing whitespace on a line is ignored.

## Comments

A `#` character that is preceded by whitespace (or is the first non-whitespace
character on a line) begins a comment that extends to the end of the line.

```
# This is a comment
name: Alice  # inline comment
```

## Token types

| Token | Pattern | Example |
|---|---|---|
| `IDENT` | `[A-Za-z_][A-Za-z0-9_-]*` | `name`, `my-key` |
| `QUOTED_STR` | `"` … `"` with `\"` and `\\` escapes | `"hello world"` |
| `INTEGER` | `-?[0-9]+` | `42`, `-7` |
| `FLOAT` | `-?[0-9]+\.[0-9]+` | `3.14`, `-0.5` |
| `BOOL` | `true` \| `false` | `true` |
| `NULL` | `null` \| `~` | `null` |
| `COLON` | `:` followed by space or EOL | `: ` |
| `DASH` | `-` at start of a non-blank line (after indent) | `- ` |
| `NEWLINE` | end of line | |
| `INDENT` | number of leading spaces on a non-blank line | |

## Reserved keywords

`true`, `false`, `null` are reserved and may not be used as bare keys unless
quoted.

## String escapes (inside `"…"`)

| Escape | Meaning |
|---|---|
| `\\` | backslash |
| `\"` | double-quote |
| `\n` | newline |
| `\t` | tab |
| `\uXXXX` | Unicode code point |
