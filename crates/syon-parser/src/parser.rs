use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::ast::{Document, MappingEntry, SequenceItem, SyonFile, Value};
use crate::error::SyonError;

#[derive(Parser)]
#[grammar = "src/grammar.pest"]
pub struct SyonParser;

// ---------------------------------------------------------------------------
// Forbidden-construct pre-flight scan
// ---------------------------------------------------------------------------

fn preflight(input: &str) -> Result<(), SyonError> {
    for (i, line) in input.lines().enumerate() {
        let ln = i + 1;
        let t = line.trim_start();

        if t == "---" || t.starts_with("--- ") || t.starts_with("---\t") {
            return Err(SyonError::Forbidden(format!(
                "line {ln}: `---` document-start marker is not allowed in SYON"
            )));
        }
        if t == "..." || t.starts_with("... ") {
            return Err(SyonError::Forbidden(format!(
                "line {ln}: `...` document-end marker is not allowed in SYON"
            )));
        }
        if t == "?" || t.starts_with("? ") {
            return Err(SyonError::Forbidden(format!(
                "line {ln}: complex key `?` is not allowed in SYON"
            )));
        }

        // Literal block delimiters `[[[` / `]]]` are not flow collections.
        if t == "[[[" || t == "]]]" {
            continue;
        }

        // Scan for forbidden inline constructs outside double-quoted strings.
        let bytes = t.as_bytes();
        let mut in_dq = false;
        let mut esc = false;
        let mut i_b = 0usize;
        while i_b < bytes.len() {
            if esc {
                esc = false;
                i_b += 1;
                continue;
            }
            match bytes[i_b] {
                b'\\' if in_dq => esc = true,
                b'"' => in_dq = !in_dq,
                b'!' if !in_dq => {
                    return Err(SyonError::Forbidden(format!(
                        "line {ln}: tag `!` / `!!` is not allowed in SYON"
                    )));
                }
                b'&' if !in_dq => {
                    // Only forbidden when followed by alphanumeric (anchor syntax)
                    if bytes.get(i_b + 1).map(|b| b.is_ascii_alphanumeric()).unwrap_or(false) {
                        return Err(SyonError::Forbidden(format!(
                            "line {ln}: anchor `&name` is not allowed in SYON"
                        )));
                    }
                }
                b'*' if !in_dq => {
                    if bytes.get(i_b + 1).map(|b| b.is_ascii_alphanumeric()).unwrap_or(false) {
                        // Only forbidden at value position (after `: ` or `- `)
                        let prefix = &t[..i_b];
                        let trimmed = prefix.trim_end();
                        if trimmed.ends_with(':') || trimmed.ends_with('-') || trimmed.is_empty() {
                            return Err(SyonError::Forbidden(format!(
                                "line {ln}: alias `*name` is not allowed in SYON"
                            )));
                        }
                    }
                }
                b'{' | b'[' if !in_dq => {
                    // Only forbidden at value positions
                    let prefix = &t[..i_b];
                    let trimmed = prefix.trim_end();
                    if trimmed.is_empty() || trimmed.ends_with(':') || trimmed.ends_with('-') {
                        let ch = bytes[i_b] as char;
                        return Err(SyonError::Forbidden(format!(
                            "line {ln}: flow collection `{ch}` is not allowed in SYON"
                        )));
                    }
                }
                _ => {}
            }
            i_b += 1;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Flat line representation after pest tokenisation
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Line {
    Comment { indent: usize, text: String },
    KeyValue { indent: usize, key: String, value: Option<LineValue>, trailing: Option<String> },
    ListItem { indent: usize, value: Option<LineValue>, trailing: Option<String> },
    LiteralBlock { indent: usize, content: String },
    FenceOpen { path: String, format: String },
    FenceClose,
}

#[derive(Debug)]
enum LineValue {
    Scalar(String),
    Literal(String),
}

// ---------------------------------------------------------------------------
// Turn the flat pest output into Line structs
// ---------------------------------------------------------------------------

fn collect_lines(input: &str) -> Result<Vec<Line>, SyonError> {
    let pairs = SyonParser::parse(Rule::document, input).map_err(|e| {
        SyonError::Syntax(format!("{e}"))
    })?;

    let mut lines = Vec::new();

    for pair in pairs.into_iter().next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::comment_line => {
                let mut indent = 0usize;
                let mut text = String::new();
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::indent => indent = inner.as_str().len(),
                        Rule::comment_text => text = inner.as_str().to_owned(),
                        _ => {}
                    }
                }
                lines.push(Line::Comment { indent, text });
            }

            Rule::key_value => {
                let mut indent = 0usize;
                let mut key = String::new();
                let mut value: Option<LineValue> = None;
                let mut trailing: Option<String> = None;
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::indent => indent = inner.as_str().len(),
                        Rule::key_body => key = inner.as_str().to_owned(),
                        Rule::inline_value => {
                            value = Some(parse_inline_value(inner));
                        }
                        Rule::inline_comment => {
                            trailing = Some(extract_comment_text(inner));
                        }
                        _ => {}
                    }
                }
                // Validate key doesn't start with operator symbols
                let k = key.trim_start();
                if k.starts_with(':') || k.starts_with('-') || k.starts_with('#') {
                    return Err(SyonError::Syntax(format!(
                        "key {:?} must not start with an operator symbol", key
                    )));
                }
                lines.push(Line::KeyValue { indent, key, value, trailing });
            }

            Rule::list_item => {
                let mut indent = 0usize;
                let mut value: Option<LineValue> = None;
                let mut trailing: Option<String> = None;
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::indent => indent = inner.as_str().len(),
                        Rule::inline_value => {
                            value = Some(parse_inline_value(inner));
                        }
                        Rule::inline_comment => {
                            trailing = Some(extract_comment_text(inner));
                        }
                        _ => {}
                    }
                }
                lines.push(Line::ListItem { indent, value, trailing });
            }

            Rule::literal_block => {
                // literal_block at top level (indent 0)
                let content = extract_literal_content(pair);
                lines.push(Line::LiteralBlock { indent: 0, content });
            }

            Rule::doc_fence_open => {
                let mut path = String::new();
                let mut format = String::new();
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::fence_path => path = inner.as_str().to_owned(),
                        Rule::fence_format => format = inner.as_str().to_owned(),
                        _ => {}
                    }
                }
                lines.push(Line::FenceOpen { path, format });
            }

            Rule::doc_fence_close => {
                lines.push(Line::FenceClose);
            }

            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(lines)
}

fn parse_inline_value(pair: Pair<Rule>) -> LineValue {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::literal_block => {
                return LineValue::Literal(extract_literal_content(inner));
            }
            Rule::scalar_value => {
                return LineValue::Scalar(extract_scalar(inner));
            }
            _ => {}
        }
    }
    LineValue::Scalar(String::new())
}

fn extract_scalar(pair: Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::dq_scalar => {
                let s = inner.as_str();
                // Strip surrounding quotes and unescape
                return unescape_dq(&s[1..s.len() - 1]);
            }
            Rule::plain_scalar => {
                return inner.as_str().trim_end().to_owned();
            }
            _ => {}
        }
    }
    String::new()
}

fn unescape_dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(c) => { out.push('\\'); out.push(c); }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_literal_content(pair: Pair<Rule>) -> String {
    let mut content = String::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::literal_content {
            for raw_line in inner.into_inner() {
                if raw_line.as_rule() == Rule::literal_raw_line {
                    content.push_str(raw_line.as_str());
                }
            }
        }
    }
    // Remove trailing newline added by the grammar
    if content.ends_with('\n') {
        content.pop();
    }
    content
}

fn extract_comment_text(pair: Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::inline_comment_text {
            return inner.as_str().trim_end().to_owned();
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Build AST from flat Line list using indentation stack
// ---------------------------------------------------------------------------

struct Builder<'a> {
    lines: &'a [Line],
    pos: usize,
}

impl<'a> Builder<'a> {
    fn new(lines: &'a [Line]) -> Self {
        Self { lines, pos: 0 }
    }

    fn peek_indent(&self) -> Option<usize> {
        self.lines.get(self.pos).map(|l| match l {
            Line::Comment { indent, .. } => *indent,
            Line::KeyValue { indent, .. } => *indent,
            Line::ListItem { indent, .. } => *indent,
            Line::LiteralBlock { indent, .. } => *indent,
            _ => 0,
        })
    }

    fn peek_is_fence(&self) -> bool {
        matches!(self.lines.get(self.pos), Some(Line::FenceOpen { .. }) | Some(Line::FenceClose))
    }

    /// Collect pending comment lines at or above `min_indent`.
    fn collect_comments(&mut self, min_indent: usize) -> Vec<String> {
        let mut out = Vec::new();
        while let Some(Line::Comment { indent, text }) = self.lines.get(self.pos) {
            if *indent < min_indent {
                break;
            }
            out.push(text.clone());
            self.pos += 1;
        }
        out
    }

    /// Parse a block of lines all sharing `expected_indent`, returning a Value.
    /// Returns None if there are no applicable lines at this indent.
    fn parse_block(&mut self, expected_indent: usize) -> Result<Option<Value>, SyonError> {
        // Peek at the first real (non-comment) line
        let save = self.pos;
        // Skip comments temporarily to see what kind of block follows
        let mut scan = self.pos;
        while let Some(Line::Comment { .. }) = self.lines.get(scan) {
            scan += 1;
        }
        match self.lines.get(scan) {
            None | Some(Line::FenceOpen { .. }) | Some(Line::FenceClose) => return Ok(None),
            Some(Line::KeyValue { indent, .. }) if *indent == expected_indent => {
                return Ok(Some(self.parse_mapping(expected_indent)?));
            }
            Some(Line::ListItem { indent, .. }) if *indent == expected_indent => {
                return Ok(Some(self.parse_sequence(expected_indent)?));
            }
            Some(Line::LiteralBlock { indent, content }) if *indent == expected_indent => {
                let content = content.clone();
                self.pos = scan + 1;
                return Ok(Some(Value::LiteralBlock(content)));
            }
            Some(Line::KeyValue { indent, .. }) | Some(Line::ListItem { indent, .. })
                if *indent != expected_indent =>
            {
                // Different indent level — not our block
                let _ = save;
                return Ok(None);
            }
            _ => return Ok(None),
        }
    }

    fn parse_mapping(&mut self, indent: usize) -> Result<Value, SyonError> {
        let mut entries: Vec<MappingEntry> = Vec::new();

        loop {
            let leading_comments = self.collect_comments(indent);

            match self.lines.get(self.pos) {
                Some(Line::KeyValue { indent: kv_indent, .. }) if *kv_indent == indent => {}
                _ => {
                    // Put comment lines back? No — they're consumed. But if no
                    // key follows, these are "trailing" block comments we discard
                    // from the mapping (they'd belong to a parent).
                    // Re-wind comment consumption if nothing followed:
                    if !leading_comments.is_empty() {
                        // Back up: rewind past the consumed comments
                        self.pos -= leading_comments.len();
                    }
                    break;
                }
            }

            if let Some(Line::KeyValue { key, value, trailing, indent: _ }) =
                self.lines.get(self.pos)
            {
                let key = key.clone();
                let inline_val = value.as_ref().map(|v| match v {
                    LineValue::Scalar(s) => Value::Scalar(s.clone()),
                    LineValue::Literal(s) => Value::LiteralBlock(s.clone()),
                });
                let trailing_comment = trailing.clone();
                self.pos += 1;

                // Check for a child block at indent+1 (or more)
                let child_indent = self.peek_indent();
                let child_value = if let Some(ci) = child_indent {
                    if ci > indent && !self.peek_is_fence() {
                        self.parse_block(ci)?
                    } else {
                        None
                    }
                } else {
                    None
                };

                let value = match (inline_val, child_value) {
                    (_, Some(child)) => child,
                    (Some(iv), None) => iv,
                    (None, None) => Value::Scalar(String::new()),
                };

                // Duplicate key check
                if entries.iter().any(|e| e.key == key) {
                    return Err(SyonError::Syntax(format!("duplicate key {:?}", key)));
                }

                entries.push(MappingEntry {
                    key,
                    value,
                    leading_comments,
                    trailing_comment,
                });
            }
        }

        Ok(Value::Mapping(entries))
    }

    fn parse_sequence(&mut self, indent: usize) -> Result<Value, SyonError> {
        let mut items: Vec<SequenceItem> = Vec::new();

        loop {
            let leading_comments = self.collect_comments(indent);

            match self.lines.get(self.pos) {
                Some(Line::ListItem { indent: li_indent, .. }) if *li_indent == indent => {}
                _ => {
                    if !leading_comments.is_empty() {
                        self.pos -= leading_comments.len();
                    }
                    break;
                }
            }

            if let Some(Line::ListItem { value, trailing, indent: _ }) = self.lines.get(self.pos) {
                let inline_val = value.as_ref().map(|v| match v {
                    LineValue::Scalar(s) => Value::Scalar(s.clone()),
                    LineValue::Literal(s) => Value::LiteralBlock(s.clone()),
                });
                let trailing_comment = trailing.clone();
                self.pos += 1;

                let child_indent = self.peek_indent();
                let child_value = if let Some(ci) = child_indent {
                    if ci > indent && !self.peek_is_fence() {
                        self.parse_block(ci)?
                    } else {
                        None
                    }
                } else {
                    None
                };

                let value = match (inline_val, child_value) {
                    (_, Some(child)) => child,
                    (Some(iv), None) => iv,
                    (None, None) => Value::Scalar(String::new()),
                };

                items.push(SequenceItem {
                    value,
                    leading_comments,
                    trailing_comment,
                });
            }
        }

        Ok(Value::Sequence(items))
    }

    /// Parse the top-level document body (indent 0 or first encountered indent).
    fn parse_document_body(&mut self) -> Result<Value, SyonError> {
        // Find the first non-comment indent without consuming comments — let
        // parse_mapping / parse_sequence collect them as leading_comments.
        let mut scan = self.pos;
        while let Some(Line::Comment { .. }) = self.lines.get(scan) {
            scan += 1;
        }

        let Some(first_indent) = (match self.lines.get(scan) {
            Some(Line::KeyValue { indent, .. })
            | Some(Line::ListItem { indent, .. })
            | Some(Line::LiteralBlock { indent, .. }) => Some(*indent),
            _ => None,
        }) else {
            return Ok(Value::Mapping(Vec::new()));
        };

        if self.peek_is_fence() {
            return Ok(Value::Mapping(Vec::new()));
        }

        let block = self.parse_block(first_indent)?;
        Ok(block.unwrap_or(Value::Mapping(Vec::new())))
    }

    /// Consume lines up to (but not including) the next FenceClose, building a Document.
    fn parse_fenced_document(
        &mut self,
        path: String,
        format: String,
    ) -> Result<Document, SyonError> {
        let body = self.parse_document_body()?;
        // Consume FenceClose if present
        if matches!(self.lines.get(self.pos), Some(Line::FenceClose)) {
            self.pos += 1;
        }
        Ok(Document { path: Some(path), format: Some(format), body })
    }
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse a SYON source string into a [`SyonFile`].
pub fn parse(input: &str) -> Result<SyonFile, SyonError> {
    preflight(input)?;

    let lines = collect_lines(input)?;
    let mut builder = Builder::new(&lines);
    let mut documents: Vec<Document> = Vec::new();

    while builder.pos < builder.lines.len() {
        match builder.lines.get(builder.pos) {
            Some(Line::FenceOpen { path, format }) => {
                let path = path.clone();
                let format = format.clone();
                builder.pos += 1;
                let doc = builder.parse_fenced_document(path, format)?;
                documents.push(doc);
            }
            Some(Line::FenceClose) => {
                // Stray close — skip
                builder.pos += 1;
            }
            _ => {
                // Main (unfenced) document
                let body = builder.parse_document_body()?;
                documents.push(Document { path: None, format: None, body });
            }
        }
    }

    if documents.is_empty() {
        documents.push(Document { path: None, format: None, body: Value::Mapping(Vec::new()) });
    }

    Ok(SyonFile { documents })
}

/// Convenience: parse and return the first document's body.
pub fn parse_document(input: &str) -> Result<crate::ast::Document, SyonError> {
    let mut file = parse(input)?;
    Ok(file.documents.remove(0))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;

    // --- Spacing rule: colon ---

    #[test]
    fn colon_space_is_key_separator() {
        let doc = parse_document("key: value\n").unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries[0].key, "key");
                assert_eq!(entries[0].value, Value::Scalar("value".into()));
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }

    #[test]
    fn colon_without_space_is_literal() {
        // "https://example.com" — the `:` is not followed by a space so it's literal
        let doc = parse_document("url: https://example.com\n").unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries[0].key, "url");
                assert_eq!(entries[0].value, Value::Scalar("https://example.com".into()));
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }

    // --- Spacing rule: dash ---

    #[test]
    fn dash_space_is_list_item() {
        let doc = parse_document("- alpha\n- beta\n").unwrap();
        match &doc.body {
            Value::Sequence(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].value, Value::Scalar("alpha".into()));
                assert_eq!(items[1].value, Value::Scalar("beta".into()));
            }
            other => panic!("expected Sequence, got {other:?}"),
        }
    }

    #[test]
    fn dash_without_space_is_literal() {
        // "-draft" as a value should not become a list item
        let doc = parse_document("tag: -draft\n").unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries[0].value, Value::Scalar("-draft".into()));
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }

    // --- Spacing rule: hash ---

    #[test]
    fn hash_space_is_comment() {
        // A comment-only document body should be empty mapping
        let doc = parse_document("# top comment\nkey: val\n").unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].key, "key");
                // Comment is attached as leading comment on the entry
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }

    #[test]
    fn hash_without_space_is_literal_value() {
        // "abc#123" — `#` not preceded by space, so it's part of the value
        let doc = parse_document("id: abc#123\n").unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries[0].value, Value::Scalar("abc#123".into()));
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }

    // --- Literal block ---

    #[test]
    fn literal_block_roundtrip() {
        let input = "[[[\nline one\nline two\n]]]\n";
        let doc = parse_document(input).unwrap();
        match &doc.body {
            Value::LiteralBlock(s) => {
                assert!(s.contains("line one"), "got: {s:?}");
                assert!(s.contains("line two"), "got: {s:?}");
            }
            other => panic!("expected LiteralBlock, got {other:?}"),
        }
    }

    // --- Forbidden constructs ---

    #[test]
    fn reject_yaml_tag() {
        let err = parse_document("key: !!str value\n").unwrap_err().to_string();
        assert!(err.contains("tag") || err.contains("!"), "got: {err}");
    }

    #[test]
    fn reject_anchor() {
        let err = parse_document("key: &anchor value\n").unwrap_err().to_string();
        assert!(err.contains("anchor") || err.contains("&"), "got: {err}");
    }

    #[test]
    fn reject_alias() {
        let err = parse_document("a: &anc val\nb: *anc\n").unwrap_err().to_string();
        assert!(err.contains("alias") || err.contains("anchor") || err.contains("*"), "got: {err}");
    }

    #[test]
    fn reject_flow_mapping() {
        let err = parse_document("{key: value}\n").unwrap_err().to_string();
        assert!(err.contains("flow") || err.contains("{"), "got: {err}");
    }

    #[test]
    fn reject_flow_sequence() {
        let err = parse_document("key: [a, b]\n").unwrap_err().to_string();
        assert!(err.contains("flow") || err.contains("["), "got: {err}");
    }

    #[test]
    fn reject_explicit_document_start() {
        assert!(parse_document("---\nkey: value\n").is_err());
    }

    #[test]
    fn reject_complex_key() {
        assert!(parse_document("? complex key\n: value\n").is_err());
    }

    // --- Nested mapping ---

    #[test]
    fn nested_mapping_parses() {
        let input = "outer:\n  inner: value\n";
        let doc = parse_document(input).unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries[0].key, "outer");
                match &entries[0].value {
                    Value::Mapping(inner) => {
                        assert_eq!(inner[0].key, "inner");
                        assert_eq!(inner[0].value, Value::Scalar("value".into()));
                    }
                    other => panic!("expected inner Mapping, got {other:?}"),
                }
            }
            other => panic!("expected outer Mapping, got {other:?}"),
        }
    }

    // --- Multi-document fence ---

    #[test]
    fn multi_document_fence_separates_documents() {
        let input = "```config.json\nkey: value\n```\n";
        let file = parse(input).unwrap();
        let fenced = file.documents.iter().find(|d| d.path.is_some()).unwrap();
        assert_eq!(fenced.path.as_deref(), Some("config"));
        assert_eq!(fenced.format.as_deref(), Some("json"));
    }

    // --- Duplicate key rejection ---

    #[test]
    fn reject_duplicate_keys() {
        let err = parse_document("a: 1\na: 2\n").unwrap_err().to_string();
        assert!(err.contains("duplicate"), "got: {err}");
    }

    // --- Comment attachment ---

    #[test]
    fn leading_comment_attached_to_entry() {
        let input = "# section header\nkey: value\n";
        let doc = parse_document(input).unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert!(!entries[0].leading_comments.is_empty(),
                    "expected leading comments on entry");
                assert_eq!(entries[0].leading_comments[0], "section header");
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }

    #[test]
    fn trailing_comment_attached_to_entry() {
        let input = "key: value # side note\n";
        let doc = parse_document(input).unwrap();
        match &doc.body {
            Value::Mapping(entries) => {
                assert_eq!(entries[0].trailing_comment.as_deref(), Some("side note"));
            }
            other => panic!("expected Mapping, got {other:?}"),
        }
    }
}
