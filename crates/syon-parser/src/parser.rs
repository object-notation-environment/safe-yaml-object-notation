use std::borrow::Cow;

use saphyr_parser::{Event, Parser, Span, SpannedEventReceiver, Tag};

use crate::ast::{Comment, Document, Mapping, MappingEntry, Sequence, Value};

/// A parse error returned by [`parse_document`].
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// A YAML construct that SYON explicitly forbids.
    Forbidden(String),
    /// A low-level scanner / syntax error.
    Syntax(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Forbidden(msg) => write!(f, "forbidden: {msg}"),
            ParseError::Syntax(msg) => write!(f, "syntax error: {msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Preflight checks on raw text
// ---------------------------------------------------------------------------

/// Scan the raw source for forbidden constructs that are detectable at the
/// text level before saphyr-parser runs.
///
/// saphyr-parser 0.0.6 does not carry block/flow style on `MappingStart` /
/// `SequenceStart` events, so flow-collection detection must happen here.
fn preflight(input: &str) -> Result<(), ParseError> {
    for (lineno, line) in input.lines().enumerate() {
        let ln = lineno + 1;
        let trimmed = line.trim_start();

        // Forbidden: explicit document-start directive `---`.
        if trimmed == "---" || trimmed.starts_with("--- ") {
            return Err(ParseError::Forbidden(format!(
                "line {ln}: explicit document start `---` is not allowed in SYON"
            )));
        }

        // Forbidden: complex mapping key `? `.
        if trimmed.starts_with("? ") || trimmed == "?" {
            return Err(ParseError::Forbidden(format!(
                "line {ln}: complex key `?` is not allowed in SYON"
            )));
        }

        // Forbidden: flow collections — `{` or `[` as the first character of a
        // value (outside double-quoted strings).  We scan from start, skipping
        // quoted regions.
        let bytes = trimmed.as_bytes();
        let mut in_dq = false;
        let mut escape_next = false;
        let mut after_colon_space = trimmed.is_empty(); // treat line-start as value position
        // Start position counts as a value position (e.g. `{…}` as whole document).
        let mut value_position = true;

        for (i, &b) in bytes.iter().enumerate() {
            if escape_next {
                escape_next = false;
                value_position = false;
                continue;
            }
            match b {
                b'\\' if in_dq => escape_next = true,
                b'"' => {
                    in_dq = !in_dq;
                    value_position = false;
                }
                b'{' | b'[' if !in_dq && value_position => {
                    let ch = b as char;
                    return Err(ParseError::Forbidden(format!(
                        "line {ln} col {}: flow collection `{ch}` is not allowed in SYON",
                        i + 1
                    )));
                }
                b':' if !in_dq => {
                    // `: ` — next non-space character is a value position.
                    if bytes.get(i + 1).copied() == Some(b' ')
                        || bytes.get(i + 1).is_none()
                    {
                        after_colon_space = true;
                        value_position = false;
                    } else {
                        value_position = false;
                    }
                }
                b' ' | b'\t' if !in_dq => {
                    if after_colon_space {
                        value_position = true;
                        after_colon_space = false;
                    } else {
                        value_position = false;
                    }
                }
                _ => {
                    after_colon_space = false;
                    value_position = false;
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Event receiver
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Frame {
    Mapping {
        entries: Vec<MappingEntry>,
        pending_key: Option<String>,
        pending_leading: Vec<Comment>,
    },
    Sequence {
        items: Vec<Value>,
    },
}

struct SyonReceiver {
    stack: Vec<Frame>,
    #[allow(dead_code)]
    /// Comments extracted from the source (line-number, text).
    comments: Vec<(usize, String)>,
    result: Option<Value>,
    error: Option<ParseError>,
}

impl SyonReceiver {
    fn new(comments: Vec<(usize, String)>) -> Self {
        Self {
            stack: Vec::new(),
            comments,
            result: None,
            error: None,
        }
    }

    fn push_value(&mut self, v: Value) {
        if let Some(frame) = self.stack.last_mut() {
            match frame {
                Frame::Mapping {
                    entries,
                    pending_key,
                    pending_leading,
                } => {
                    if let Some(key) = pending_key.take() {
                        entries.push(MappingEntry {
                            key,
                            value: v,
                            leading_comments: std::mem::take(pending_leading),
                            trailing_comment: None,
                        });
                    } else {
                        // This value IS the key (scalars only).
                        if let Value::Scalar(k) = v {
                            *pending_key = Some(k);
                        } else {
                            self.error = Some(ParseError::Forbidden(
                                "mapping keys must be plain scalars".into(),
                            ));
                        }
                    }
                }
                Frame::Sequence { items } => {
                    items.push(v);
                }
            }
        } else {
            // Top-level value.
            self.result = Some(v);
        }
    }

    fn check_anchor(anchor_id: usize) -> Option<ParseError> {
        if anchor_id != 0 {
            Some(ParseError::Forbidden(
                "anchor `&name` is not allowed in SYON".into(),
            ))
        } else {
            None
        }
    }

    fn check_tag(tag: &Option<Cow<'_, Tag>>) -> Option<ParseError> {
        if tag.is_some() {
            Some(ParseError::Forbidden(
                "tag `!tag` / `!!type` is not allowed in SYON".into(),
            ))
        } else {
            None
        }
    }
}

impl<'input> SpannedEventReceiver<'input> for SyonReceiver {
    fn on_event(&mut self, ev: Event<'input>, _span: Span) {
        if self.error.is_some() {
            return;
        }

        match ev {
            Event::StreamStart | Event::StreamEnd | Event::DocumentEnd | Event::Nothing => {}

            Event::DocumentStart(explicit) => {
                if explicit {
                    self.error = Some(ParseError::Forbidden(
                        "explicit document start `---` is not allowed in SYON".into(),
                    ));
                }
            }

            Event::Alias(_) => {
                self.error = Some(ParseError::Forbidden(
                    "alias `*name` is not allowed in SYON".into(),
                ));
            }

            Event::Scalar(value, _style, anchor_id, ref tag) => {
                if let Some(e) = Self::check_anchor(anchor_id) {
                    self.error = Some(e);
                    return;
                }
                if let Some(e) = Self::check_tag(tag) {
                    self.error = Some(e);
                    return;
                }
                self.push_value(Value::Scalar(value.into_owned()));
            }

            Event::MappingStart(anchor_id, ref tag) => {
                if let Some(e) = Self::check_anchor(anchor_id) {
                    self.error = Some(e);
                    return;
                }
                if let Some(e) = Self::check_tag(tag) {
                    self.error = Some(e);
                    return;
                }
                self.stack.push(Frame::Mapping {
                    entries: Vec::new(),
                    pending_key: None,
                    pending_leading: Vec::new(),
                });
            }

            Event::MappingEnd => {
                if let Some(Frame::Mapping {
                    entries,
                    pending_key: _,
                    pending_leading: _,
                }) = self.stack.pop()
                {
                    // Check for duplicate keys.
                    let mut seen = std::collections::HashSet::new();
                    for e in &entries {
                        if !seen.insert(e.key.clone()) {
                            self.error = Some(ParseError::Syntax(format!(
                                "duplicate key {:?}",
                                e.key
                            )));
                            return;
                        }
                    }
                    self.push_value(Value::Mapping(Mapping { entries }));
                }
            }

            Event::SequenceStart(anchor_id, ref tag) => {
                if let Some(e) = Self::check_anchor(anchor_id) {
                    self.error = Some(e);
                    return;
                }
                if let Some(e) = Self::check_tag(tag) {
                    self.error = Some(e);
                    return;
                }
                self.stack.push(Frame::Sequence { items: Vec::new() });
            }

            Event::SequenceEnd => {
                if let Some(Frame::Sequence { items }) = self.stack.pop() {
                    self.push_value(Value::Sequence(Sequence { items }));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Comment extraction
// ---------------------------------------------------------------------------

/// Scan the raw SYON source and collect `(1-based line, comment text)` pairs.
///
/// Applies the spacing rule: `#` is structural only when preceded by
/// whitespace (or at column 0) and followed by a space or end-of-line.
fn extract_comments(input: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (lineno, line) in input.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut in_dq = false;
        let mut escape_next = false;
        for (i, &b) in bytes.iter().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }
            match b {
                b'\\' if in_dq => escape_next = true,
                b'"' => in_dq = !in_dq,
                b'#' if !in_dq => {
                    let preceded_by_ws = i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t';
                    let followed_by_ws_or_eol = i + 1 >= bytes.len()
                        || bytes[i + 1] == b' '
                        || bytes[i + 1] == b'\t';
                    if preceded_by_ws && followed_by_ws_or_eol {
                        let text = line[i + 1..].trim().to_owned();
                        out.push((lineno + 1, text));
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Literal block extraction
// ---------------------------------------------------------------------------

/// Check whether `input` is a standalone literal block (`[[[ … ]]]`).
fn try_parse_literal_block(input: &str) -> Option<Value> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.first().map(|l| l.trim()) == Some("[[[") {
        let close = lines.iter().rposition(|l| l.trim() == "]]]")?;
        let body = lines[1..close].join("\n");
        return Some(Value::LiteralBlock(body));
    }
    None
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a SYON source string into a [`Document`].
///
/// Returns [`ParseError::Forbidden`] if any YAML construct that SYON forbids
/// is encountered, and [`ParseError::Syntax`] for malformed input.
pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    // 1. Check for standalone literal block (`[[[ … ]]]`).
    if let Some(lit) = try_parse_literal_block(input.trim()) {
        return Ok(Document {
            body: lit,
            trailing_comments: Vec::new(),
        });
    }

    // 2. Preflight scan for constructs that can't be detected from events.
    preflight(input)?;

    // 3. Extract comments before saphyr-parser strips them.
    let comments = extract_comments(input);

    // 4. Run saphyr-parser event loop.
    let mut recv = SyonReceiver::new(comments.clone());
    let mut parser = Parser::new_from_str(input);
    parser
        .load(&mut recv, true)
        .map_err(|e| ParseError::Syntax(e.to_string()))?;

    if let Some(err) = recv.error {
        return Err(err);
    }

    // 5. Attach trailing (document-level) comments — those whose line number
    //    is after the last key/value event.  For this scaffold we attach all
    //    extracted comments to the document root; a full implementation would
    //    thread span info through the receiver to place them precisely.
    let trailing_comments: Vec<Comment> = comments
        .into_iter()
        .map(|(_, text)| Comment { text })
        .collect();

    let body = recv
        .result
        .unwrap_or(Value::Mapping(Mapping { entries: Vec::new() }));

    Ok(Document {
        body,
        trailing_comments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;

    // --- Basic parsing ---

    #[test]
    fn parse_scalar() {
        let doc = parse_document("hello").unwrap();
        assert_eq!(doc.body, Value::Scalar("hello".into()));
    }

    #[test]
    fn parse_block_mapping() {
        let doc = parse_document("name: Alice\nage: 30\n").unwrap();
        if let Value::Mapping(m) = &doc.body {
            assert_eq!(m.entries[0].key, "name");
            assert_eq!(m.entries[0].value, Value::Scalar("Alice".into()));
            assert_eq!(m.entries[1].key, "age");
            assert_eq!(m.entries[1].value, Value::Scalar("30".into()));
        } else {
            panic!("expected Mapping");
        }
    }

    #[test]
    fn parse_block_sequence() {
        let doc = parse_document("- alpha\n- beta\n").unwrap();
        if let Value::Sequence(seq) = &doc.body {
            assert_eq!(seq.items.len(), 2);
            assert_eq!(seq.items[0], Value::Scalar("alpha".into()));
        } else {
            panic!("expected Sequence");
        }
    }

    // --- Forbidden construct rejection ---

    #[test]
    fn reject_yaml_tag() {
        let result = parse_document("key: !!str value\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("tag"), "expected 'tag' in: {msg}");
    }

    #[test]
    fn reject_anchor() {
        let result = parse_document("key: &anchor value\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("anchor"), "expected 'anchor' in: {msg}");
    }

    #[test]
    fn reject_alias() {
        let result = parse_document("a: &anc val\nb: *anc\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("alias") || msg.contains("anchor"), "got: {msg}");
    }

    #[test]
    fn reject_flow_mapping() {
        let result = parse_document("{key: value}\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("flow"), "expected 'flow' in: {msg}");
    }

    #[test]
    fn reject_flow_sequence() {
        let result = parse_document("key: [a, b]\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("flow"), "expected 'flow' in: {msg}");
    }

    #[test]
    fn reject_explicit_document_start() {
        let result = parse_document("---\nkey: value\n");
        assert!(result.is_err());
    }

    #[test]
    fn reject_complex_key() {
        let result = parse_document("? complex key\n: value\n");
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_keys() {
        let result = parse_document("a: 1\na: 2\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("duplicate"), "expected 'duplicate' in: {msg}");
    }

    // --- Literal block ---

    #[test]
    fn parse_standalone_literal_block() {
        let input = "[[[\nline one\nline two\n]]]\n";
        let doc = parse_document(input).unwrap();
        assert!(
            matches!(&doc.body, Value::LiteralBlock(s) if s.contains("line one")),
            "expected LiteralBlock, got {:?}",
            doc.body
        );
    }

    // --- Comment extraction ---

    #[test]
    fn leading_comment_extracted() {
        let input = "# top comment\nkey: val\n";
        let doc = parse_document(input).unwrap();
        // Comments are currently attached to trailing_comments at document level.
        assert!(!doc.trailing_comments.is_empty());
        assert_eq!(doc.trailing_comments[0].text, "top comment");
    }

    #[test]
    fn inline_comment_extracted() {
        let input = "key: value # side note\n";
        let doc = parse_document(input).unwrap();
        assert!(doc.trailing_comments.iter().any(|c| c.text == "side note"));
    }

    #[test]
    fn hash_without_space_not_a_comment() {
        let input = "id: abc#123\n";
        let doc = parse_document(input).unwrap();
        // "abc#123" should be the scalar value, not split into value + comment.
        assert!(doc.trailing_comments.is_empty());
        if let Value::Mapping(m) = &doc.body {
            assert_eq!(m.entries[0].value, Value::Scalar("abc#123".into()));
        } else {
            panic!("expected Mapping");
        }
    }
}
