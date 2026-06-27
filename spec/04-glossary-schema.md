# SYON Glossary Schema Convention (v0.1.0)

## 4.1 Purpose

SYON has no built-in schema syntax. This document defines a **convention**
for using SYON as a schema for glossary and terminology entries. The
convention is itself expressed in SYON (`examples/glossary/schema.syon`),
demonstrating that SYON is expressive enough to describe its own
meta-structures using only its core features: mappings, sequences, and
literal blocks.

A **glossary entry** is a SYON document that captures the definition,
provenance, relationships, and history of a single concept or term. The
schema is the agreed-upon set of field names, types, and constraints that
every conforming entry must or may include.

---

## 4.2 Field Groups

Fields are organised into eight groups. Within each group, the presence
rules and type constraints are listed.

### 4.2.1 Identity — `term` and `abbreviation`

| Field | Type | Required |
|-------|------|----------|
| `term` | string | optional |
| `abbreviation` | string | optional |

**Constraint (one-of):** at least one of `term` or `abbreviation` must be
present. An entry may supply both.

`term` holds the full human-readable name of the concept.
`abbreviation` holds the short form or acronym.

### 4.2.2 Provenance — `id` and `version`

| Field | Type | Required |
|-------|------|----------|
| `id` | string | required |
| `version` | string | required |

`id` is a stable machine-readable identifier (e.g. `syon-001`). It is
intended for use as a cross-reference target in `relationships` mappings.

`version` records the version of the entry or of the concept it describes
(e.g. `0.9.0`). Implementations that need to detect stale cross-references
should compare the `version` at the call site against the authoritative
entry.

### 4.2.3 Definition — `description`

| Field | Type | Required |
|-------|------|----------|
| `description` | string | required |

A human-readable definition of the term. Short definitions may be plain
scalars; longer definitions should use a literal block so that paragraph
structure is preserved verbatim.

```syon
# Plain scalar (single sentence)
description: A compact binary encoding for structured data.

# Literal block (multi-line)
description: [[[
  A human-writable data serialization format that is safe (no implicit
  typing, no executable constructs), simple (a small fixed set of markers),
  and structured (keys, lists, nesting).
]]]
```

### 4.2.4 Scope — `contexts`

| Field | Type | Required | Items |
|-------|------|----------|-------|
| `contexts` | sequence | optional | string |

A list of domain tags that scope where the term applies. Tags are free-form
strings; by convention they use kebab-case (e.g. `data-formats`,
`serialization`, `one-family`).

```syon
contexts:
  - data-formats
  - serialization
```

### 4.2.5 Naming Variants — `synonyms` and `opposites`

| Field | Type | Required |
|-------|------|----------|
| `synonyms` | sequence or mapping | optional |
| `opposites` | sequence or mapping | optional |

Both fields accept either of two shapes:

**Sequence form** — a flat list of strings, used when no additional context
is needed per entry:

```syon
synonyms:
  - safe yaml subset
  - syon format
```

**Mapping form** — keys are context labels, values are the synonym or
opposite term in that context. Use this when the relationship depends on
perspective or scope:

```syon
opposites:
  full-yaml: unrestricted YAML with tags, anchors, and implicit typing
  json: no comments, no trailing commas, no multi-line strings
```

Parsers and tooling MUST accept both shapes. A field whose value is a
mapping with string values is distinguishable from a sequence by the
presence of keys.

### 4.2.6 Relationships — `relationships`

| Field | Type | Required |
|-------|------|----------|
| `relationships` | mapping | optional |

A mapping of typed edges to related glossary entries. Keys are relationship
role identifiers; values are either a single entry `id` string or a
sequence of `id` strings.

Recommended role identifiers (not exhaustive):

| Role | Meaning |
|------|---------|
| `see-also` | Related concept worth reading alongside this one |
| `member-of` | This concept belongs to the named family or group |
| `inspired-by` | This concept's design was influenced by the target |
| `supersedes` | This concept replaces the target |
| `subset-of` | This concept is a strict subset of the target |

```syon
relationships:
  see-also: yaml-001
  member-of: one-family
  inspired-by:
    - strictyaml-001
    - nestedtext-001
```

### 4.2.7 Discussion — `discussion`

| Field | Type | Required |
|-------|------|----------|
| `discussion` | file-ref | optional |

A relative file path pointing to a `.md` or `.syon` file that contains
extended discussion, worked examples, or design rationale for this entry.

**Path resolution** (Section 4.2): the path is resolved relative to the
directory containing the entry file. For example, if the entry is at
`examples/glossary/entries/syon.syon` and `discussion: syon-discussion.md`,
the resolved path is `examples/glossary/entries/syon-discussion.md`.

Absolute paths and `..` traversals beyond the repository root are
non-conformant. Implementations SHOULD reject them.

The value is a plain string scalar at the SYON parse boundary — path
resolution is a post-parse concern for the consuming application.

### 4.2.8 Change History — `history` and `changelogs`

| Field | Type | Required |
|-------|------|----------|
| `history` | mapping | optional |
| `changelogs` | mapping | optional |

**Constraint (one-of):** at most one of `history` or `changelogs` may be
present; having both is an error. Entries that record no history may omit
both.

Both fields hold a mapping of ISO-8601 date strings to change-description
strings:

```syon
history:
  2026-06-27: v0.9.0 draft published
  2026-01-15: initial entry created
```

The two key names (`history` / `changelogs`) exist because different teams
adopt different naming conventions; the constraint group ensures they remain
interchangeable.

---

## 4.3 Constraint Conventions

SYON provides no built-in validation primitives. This convention introduces
two meta-fields — `one-of-group` and `cardinality` — that are used in the
schema file to annotate fields with cross-field constraints.

### 4.3.1 `one-of` groups

A `one-of` group is a named set of fields with a mutual presence constraint.
Each field in the group carries `one-of-group: <name>` in its schema
definition. The `constraints` block at the bottom of `schema.syon` lists
the group's members and its cardinality.

| `cardinality` value | Meaning |
|---------------------|---------|
| `at-least-one` | At least one field from the group must be present |
| `at-most-one` | At most one field from the group may be present |

The schema currently defines two `one-of` groups:

```
one-of: identity   members: [term, abbreviation]   cardinality: at-least-one
one-of: changelog  members: [history, changelogs]   cardinality: at-most-one
```

### 4.3.2 `all-of` groups (reserved)

An `all-of` group would require ALL listed fields to be present whenever
any one of them is. This is reserved for future use; no `all-of` groups are
defined in the current schema. Implementations SHOULD ignore unknown rule
names gracefully.

---

## 4.4 Annotated Worked Example

The following is the complete glossary entry for SYON itself
(`examples/glossary/entries/syon.syon`), annotated line by line.

```syon
abbreviation: SYON               # identity group — satisfies at-least-one
term: Safe YAML Object Notation  # identity group — both present, both kept
id: syon-001                     # provenance — stable cross-reference id
version: 0.9.0                   # provenance — current spec version
description: [[[                 # definition — literal block for multi-line
  A human-writable data serialization format that is safe (no implicit
  typing, no executable constructs), simple (a small fixed set of markers),
  and structured (keys, lists, nesting). A member of the ONE family.
]]]
contexts:                        # scope — domain tags
  - data-formats
  - serialization
  - one-family
synonyms:                        # naming variants — sequence form
  - safe yaml subset
opposites:                       # naming variants — mapping form (label: explanation)
  full-yaml: unrestricted YAML with tags, anchors, and implicit typing
relationships:                   # graph edges
  see-also: yaml-001
  member-of: one-family
  inspired-by:                   # sequence value for a multi-target role
    - strictyaml-001
    - nestedtext-001
discussion: syon-discussion.md   # file-ref — resolved relative to entry dir
history:                         # changelog group — at-most-one satisfied
  2026-06-27: v0.9.0 draft published
```

### Key design decisions visible in this example

**Both `abbreviation` and `term` are present.** The `at-least-one`
constraint is satisfied, and providing both gives tooling a choice of how
to display the entry (short form vs. long form).

**`description` uses a literal block.** The `[[[...]]]` syntax preserves
paragraph structure without requiring escape sequences.

**`synonyms` uses sequence form, `opposites` uses mapping form.** This
demonstrates that the two shapes are not interchangeable when context labels
add value (as they do for `opposites`).

**`inspired-by` holds a sequence.** When a relationship role has multiple
targets, the value is a SYON sequence rather than a repeated key (which
would violate SYON's duplicate-key prohibition).

**`history` is present, `changelogs` is absent.** The `at-most-one`
constraint is satisfied.

---

## 4.5 `discussion` Path Resolution

The `discussion` field holds a **relative path** in the filesystem sense.
Resolution follows these rules:

1. The base directory is the directory containing the entry file itself
   (not the glossary root, not the repository root).
2. The path MUST NOT contain components that escape the repository root
   (e.g. `../../etc/passwd`). Implementations SHOULD reject such paths.
3. The file extension determines how the referenced file is parsed:
   - `.md` — Markdown, rendered or displayed as prose
   - `.syon` — a SYON document (may itself be a mapping, sequence, or
     literal block)
   - Other extensions MAY be supported by applications.
4. A missing file at the resolved path is a soft error — the entry itself
   remains valid. Implementations SHOULD warn rather than reject.

The SYON parser returns the `discussion` value as a plain `Scalar` string.
Path resolution is a post-parse, application-layer concern and is outside
the scope of the SYON grammar and semantics specifications.

---

## 4.6 Sequence vs. Mapping for `synonyms` and `opposites`

Both fields accept either a block sequence or a block mapping.

**Choose the sequence form** when the synonyms or opposites are
context-independent and a bare list is sufficient:

```syon
synonyms:
  - safe yaml subset
  - syon-format
```

**Choose the mapping form** when each entry benefits from a short label or
when the relationship is directional and the direction depends on context:

```syon
opposites:
  full-yaml: unrestricted YAML with tags, anchors, and implicit typing
  toml: configuration-first, richer type system, stricter quoting rules
```

The mapping form's values are plain strings (SYON scalars), not nested
structures. Parsers distinguish the two shapes by inspecting the parsed
`Value` variant:

- `Value::Sequence(_)` → sequence form
- `Value::Mapping(_)` → mapping form
- `Value::Scalar(_)` → single-item shorthand (treated as a one-element list)

---

## 4.7 Conformance

A SYON document is a **conforming glossary entry** if:

1. It is a valid SYON document per the grammar in `spec/02-grammar.md`.
2. It contains at least one of `term` or `abbreviation`.
3. It contains `id` and `version`.
4. It contains `description`.
5. It does not contain both `history` and `changelogs`.
6. All field values match their declared types in `examples/glossary/schema.syon`.

Implementations MAY enforce additional constraints (e.g. `id` format,
`version` semver conformance, `contexts` from a controlled vocabulary).
