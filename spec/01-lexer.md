# SYON Lexer Specification (v0.9.0)

## Encoding

SYON source files MUST be UTF-8 encoded. A file that is not valid UTF-8 MUST
be rejected with a decode error before any lexing begins.

## Line orientation

SYON is line-oriented. Lines are separated by `\n` (LF) or `\r\n` (CRLF);
both are normalised to `\n`. Trailing whitespace on any line is ignored for
structural purposes.

## Indentation

- **Spaces only.** Tabs in the indentation prefix are a lexer error.
- **No trailing tabs.** Trailing whitespace (spaces or tabs) is discarded.
- Blank lines (empty or whitespace-only) are skipped; they do not affect
  indentation tracking.
- An increase in leading spaces relative to the previous non-blank line emits
  an `Indent` token; a decrease emits a `Dedent` token.

## The spacing rule (Section 2.4)

A character is **structural** only when it is followed by a space (`U+0020`),
a tab, or an end-of-line (EOL), and (for inline markers) preceded by whitespace
or the start of the line.

| Marker | Structural form | Literal form |
|--------|----------------|--------------|
| `:`    | `: ` or `:\n`  | `:x` (no space follows) |
| `-`    | `- ` or `-\n`  | `-x` or `-1` |
| `#`    | `# ` or `#\n` at line-start or after whitespace | `#x` or `abc#123` |

This means values do **not** need quoting or escaping for these characters as
long as they are not followed by a space in a position where the marker would
be structural.

Examples:

```
url: https://example.com   # ok — `:` in the value is not followed by space
tag: -draft                # ok — `-` in the value is not preceded by structural position
id:  abc#123               # ok — `#` is not preceded by space
```

## Token types

| Token | Description |
|-------|-------------|
| `Key(String)` | Bare key preceding a structural `: ` |
| `Value(String)` | Scalar value on the same line as a key or list item |
| `ListItem` | The `- ` sequence item marker |
| `Comment(String)` | Text following a structural `# ` marker |
| `LiteralBlockOpen` | Opening `[[[` on its own line |
| `LiteralBlockClose` | Closing `]]]` on its own line |
| `DocFence { path, format }` | Opening triple-backtick fence with `path.format` info string |
| `Indent` | Indentation level increased |
| `Dedent` | Indentation level decreased |

## Key restrictions

Keys MUST NOT begin with any operator symbol (`:`, `-`, `#`). A key such as
`:bad` or `-also-bad` is a lexer error.
