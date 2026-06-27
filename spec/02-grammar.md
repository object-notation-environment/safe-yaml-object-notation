# SYON Grammar

Formal grammar in EBNF. Terminal tokens are defined in `01-lexer.md`.

```ebnf
document      = value EOF ;

value         = scalar
              | block-sequence
              | block-mapping
              | flow-sequence
              | flow-mapping ;

(* scalars *)
scalar        = NULL | BOOL | INTEGER | FLOAT | QUOTED_STR | IDENT ;

(* block collections — indentation-sensitive *)
block-sequence = { INDENT DASH value NEWLINE } ;

block-mapping  = { INDENT IDENT COLON ( value NEWLINE
                                       | NEWLINE block-collection ) } ;

block-collection = block-sequence | block-mapping ;

(* flow collections — single-line, indentation-insensitive *)
flow-sequence  = "[" [ flow-value { "," flow-value } ] "]" ;

flow-mapping   = "{" [ flow-pair  { "," flow-pair  } ] "}" ;

flow-pair      = ( IDENT | QUOTED_STR ) ":" flow-value ;

flow-value     = scalar | flow-sequence | flow-mapping ;
```

## Indentation rules

1. A **block collection** starts one indent level deeper than its parent key.
2. All items in the same collection must share the same indent.
3. An item at a shallower indent terminates the current collection.
4. The top-level value is at indent 0.

## Restrictions (vs. YAML)

- `---` and `...` document markers are **illegal**.
- `&anchor`, `*alias` are **illegal**.
- `!!tag` explicit tags are **illegal**.
- Block scalars (`|`, `>`) are **not supported** in this version.
