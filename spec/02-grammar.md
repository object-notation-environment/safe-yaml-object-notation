# SYON Grammar (v0.9.0)

## Three-block model

A SYON document is composed of three distinct block types that can appear at
any nesting level:

### Block 1 — Record (YAML block-style subset)

The primary block type. Uses indentation and the structural markers `: `, `- `,
`# ` from the lexer.

```ebnf
record      = mapping | sequence | scalar ;
mapping     = { indent key ":" SP value newline } ;
sequence    = { indent "-" SP value newline } ;
scalar      = STRING ;               (* plain or double-quoted *)
```

All content is a **string at the parse boundary** — no implicit type coercion
(see `03-semantics.md`).

The recommended implementation strategy is to use `saphyr-parser` (a YAML 1.2
event parser) on Block 1 content, filtering the event stream to reject the
forbidden construct set.

### Block 2 — Document fence

An embedded sub-document with an explicit media-type annotation. The fence is
two triple-backtick lines at **column 0**, with a `path.format` info string on
the opening line.

```
```path/to/resource.json
{ … embedded content … }
```
```

The info string MUST contain at least one `.` separator; the part after the
last `.` is the format identifier. The parser exposes this as a `DocFence`
token with `path` and `format` fields.

### Block 3 — Literal escape hatch

A verbatim, uninterpreted content block delimited by `[[[` and `]]]` on their
own lines at **column 0** (or at the current indent level for nested use).

```
[[[
any content, including YAML syntax characters
]]]
```

The parser returns the enclosed content as a `LiteralBlock(String)` value
node. Indentation is preserved exactly as written.

## Forbidden set

The following YAML constructs MUST be rejected by a conforming SYON parser:

| Construct | Why forbidden |
|-----------|---------------|
| `!tag` / `!!type` explicit tags | Introduce arbitrary typing |
| `&anchor` anchors | Enable reference cycles |
| `*alias` aliases | Enable reference cycles |
| `{…}` flow mappings | Disallowed flow style |
| `[…]` flow sequences | Disallowed flow style |
| `,` as flow separator | Part of disallowed flow style |
| `?` complex key | Not needed in the safe subset |
| `---` explicit document-start marker | No multi-document streams |
| `...` document-end marker | No multi-document streams |

## Formal grammar (EBNF excerpt)

```ebnf
document       = block-1 | block-2 | block-3 ;

(* Block 1 — YAML block style subset *)
block-1        = mapping | sequence | scalar ;
mapping        = mapping-entry { mapping-entry } ;
mapping-entry  = indent key COLON-SP value NEWLINE ;
sequence       = sequence-item { sequence-item } ;
sequence-item  = indent DASH-SP value NEWLINE ;
value          = scalar | mapping | sequence | block-3 ;
key            = IDENT ;            (* must not start with `:`, `-`, `#` *)
scalar         = plain-scalar | dq-scalar ;
plain-scalar   = CHAR+ ;            (* spacing rule applies *)
dq-scalar      = DQUOTE CHAR* DQUOTE ;

(* Block 2 — Document fence *)
block-2        = FENCE-OPEN CONTENT FENCE-CLOSE ;
FENCE-OPEN     = "```" path "." format NEWLINE ;
FENCE-CLOSE    = "```" NEWLINE ;

(* Block 3 — Literal escape hatch *)
block-3        = "[[[" NEWLINE RAW-CONTENT "]]]" NEWLINE ;
RAW-CONTENT    = { any-char } ;     (* verbatim, not interpreted *)

(* Terminals *)
COLON-SP       = ":" ( SP | NEWLINE ) ;
DASH-SP        = "-" ( SP | NEWLINE ) ;
IDENT          = CHAR+ ;
SP             = U+0020 ;
DQUOTE         = U+0022 ;
NEWLINE        = U+000A ;
```
