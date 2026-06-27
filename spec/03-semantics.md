# SYON Semantics (v0.9.0)

## Strings-only boundary

**All scalars are strings at the parse boundary.**

SYON deliberately does not perform implicit type coercion on scalar values.
A parser that encounters the token `42` MUST return it as the string `"42"`,
not as an integer. Applications that need typed values perform their own
post-parse interpretation.

This design eliminates the class of bugs caused by implicit YAML coercions
(e.g., `yes`/`no`/`on`/`off` silently becoming booleans, leading-zero integers
being interpreted as octal, etc.).

| Input token | SYON type | Notes |
|-------------|-----------|-------|
| `42` | `Scalar("42")` | Not an integer |
| `3.14` | `Scalar("3.14")` | Not a float |
| `true` | `Scalar("true")` | Not a boolean |
| `null` | `Scalar("null")` | Not null |
| `"hello"` | `Scalar("hello")` | Quotes stripped |
| `[[[…]]]` | `LiteralBlock(…)` | Verbatim string |

## Value type hierarchy

```
Value
  ├── Scalar(String)           all parsed scalars
  ├── LiteralBlock(String)     verbatim [[[ … ]]] content
  ├── Mapping(Vec<MappingEntry>)
  │     MappingEntry { key: String, value: Value,
  │                    leading_comments, trailing_comment }
  └── Sequence(Vec<Value>)
```

## Comment attachment rules (Section 3.3.1)

Comments are first-class nodes in the SYON AST. A comment is attached to the
nearest structural node according to the following rules, applied in order:

1. **Block comment** — one or more `# ` lines immediately before a key on
   their own lines (no intervening blank lines): attached as `leading_comments`
   on the following `MappingEntry`.

2. **Trailing comment** — a `# ` fragment on the same line as a key or value
   (after the value text): attached as `trailing_comment` on the `MappingEntry`
   whose key or value appears on that line.

3. **Document-trailing comment** — any comment that does not satisfy rules 1
   or 2 (e.g., comments after the last key, or at the very end of the file):
   attached to `Document.trailing_comments`.

Example:

```syon
# (1) leading comment for "name"
name: Alice   # (2) trailing comment for "name"
age: 30
# (3) document-trailing comment
```

## Path resolution (Block 2)

The `path.format` info string on a Block 2 document fence carries two pieces
of semantic information:

- **path**: a relative path identifying the logical location of the embedded
  document within the surrounding SYON tree. An empty path (omitted) means the
  document is anonymous.
- **format**: the media-type shorthand for the embedded content (e.g. `json`,
  `toml`, `md`, `txt`). The SYON parser does NOT parse the embedded content —
  it is returned as a raw string in the AST. The application is responsible for
  dispatching to the appropriate sub-parser.

## Duplicate keys

Duplicate keys within the same mapping are a **parse error**. Implementations
MUST reject such documents; silent last-wins behaviour is non-conformant.

## Error model

All errors are fatal — there is no partial-result or best-effort mode.
Implementations return either a complete `Document` or an error carrying:

- the 1-based line number,
- the 1-based column,
- a human-readable message distinguishing `Forbidden` errors (invalid YAML
  constructs) from `Syntax` errors (malformed input).
