/// SYON surface-level token produced by the first-pass tokenizer.
///
/// These tokens correspond directly to SYON's three-block surface grammar
/// before any semantic processing occurs.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A mapping key — the bare identifier before `: ` (colon-space).
    Key(String),
    /// A scalar value — the text after a `: ` separator or on a bare line.
    Value(String),
    /// A sequence item marker — `- ` (dash-space) at the start of a line.
    ListItem,
    /// A comment — the text after `# ` (hash-space), not including the marker.
    Comment(String),
    /// Opening of a verbatim literal block (`[[[`).
    LiteralBlockOpen,
    /// Closing of a verbatim literal block (`]]]`).
    LiteralBlockClose,
    /// A triple-backtick document fence with a `path.format` info string.
    DocFence { path: String, format: String },
    /// Indentation increased relative to the previous non-empty line.
    Indent,
    /// Indentation decreased relative to the previous non-empty line.
    Dedent,
}

/// Extract a comment from a raw line fragment, applying the spacing rule.
///
/// `# ` (hash followed by a space or at end-of-line) is a comment marker.
/// `#x` (hash without a following space) is treated as literal content.
fn extract_inline_comment(text: &str) -> (String, Option<String>) {
    // Walk through text looking for " # " or text that ends with " #"
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut in_quotes = false;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quotes = !in_quotes,
            b'#' if !in_quotes => {
                // Spacing rule: structural only when preceded by space or at start.
                let preceded_by_space =
                    i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t';
                let followed_by_space_or_eol =
                    i + 1 >= bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'\t';
                if preceded_by_space && followed_by_space_or_eol {
                    let content = text[..i].trim_end().to_owned();
                    let comment = text[i + 1..].trim().to_owned();
                    return (content, Some(comment));
                }
            }
            _ => {}
        }
        i += 1;
    }
    (text.to_owned(), None)
}

/// First-pass SYON tokenizer.
///
/// Converts a raw SYON source string into a flat list of `Token`s. This pass
/// handles the three-block surface grammar and the spacing rule; it does NOT
/// perform YAML event parsing — that is done by `parser.rs` on the output of
/// this stage.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut prev_indent: usize = 0;
    let mut in_literal = false;
    let mut literal_buf = String::new();

    for line in input.lines() {
        // --- Block 3: Literal block ---
        if in_literal {
            if line.trim_end() == "]]]" {
                tokens.push(Token::Value(literal_buf.clone()));
                literal_buf.clear();
                tokens.push(Token::LiteralBlockClose);
                in_literal = false;
            } else {
                literal_buf.push_str(line);
                literal_buf.push('\n');
            }
            continue;
        }

        let trimmed = line.trim_end();

        // Blank line — skip without emitting indent/dedent.
        if trimmed.trim_start().is_empty() {
            continue;
        }

        // --- Block 2: Document fence ---
        if trimmed.starts_with("```") && {
            let after = trimmed[3..].trim();
            !after.is_empty() && !after.starts_with('`')
        } {
            let info = trimmed[3..].trim();
            let (path, format) = info.rsplit_once('.').unwrap_or((info, ""));
            tokens.push(Token::DocFence {
                path: path.to_owned(),
                format: format.to_owned(),
            });
            continue;
        }
        if trimmed == "```" {
            // Closing fence — re-use DocFence with empty fields as the close marker.
            tokens.push(Token::DocFence {
                path: String::new(),
                format: String::new(),
            });
            continue;
        }

        // --- Block 3 open ---
        if trimmed.trim_start() == "[[[" {
            tokens.push(Token::LiteralBlockOpen);
            in_literal = true;
            continue;
        }

        // --- Indentation tracking ---
        let indent = line.len() - line.trim_start().len();
        if indent > prev_indent {
            tokens.push(Token::Indent);
        } else if indent < prev_indent {
            tokens.push(Token::Dedent);
        }
        prev_indent = indent;

        let content = trimmed.trim_start();

        // --- Spacing rule: `# ` comment at column-0 or after space ---
        if content.starts_with("# ") || content == "#" {
            let text = if content.len() > 2 {
                content[2..].to_owned()
            } else {
                String::new()
            };
            tokens.push(Token::Comment(text));
            continue;
        }

        // --- Spacing rule: `- ` list item ---
        if content.starts_with("- ") || content == "-" {
            tokens.push(Token::ListItem);
            let rest = if content.len() > 2 { content[2..].trim() } else { "" };
            if !rest.is_empty() {
                let (value_part, comment) = extract_inline_comment(rest);
                if !value_part.is_empty() {
                    tokens.push(Token::Value(value_part));
                }
                if let Some(c) = comment {
                    tokens.push(Token::Comment(c));
                }
            }
            continue;
        }

        // --- Spacing rule: `key: ` or `key:` mapping entry ---
        if let Some(colon_pos) = find_structural_colon(content) {
            let key = content[..colon_pos].trim().to_owned();

            // Keys must not start with operator symbols (Section 3.x).
            if key.starts_with(':') || key.starts_with('-') || key.starts_with('#') {
                return Err(format!(
                    "key {key:?} must not start with an operator symbol (`:`, `-`, `#`)"
                ));
            }

            tokens.push(Token::Key(key));
            let value_part = content[colon_pos + 1..].trim();
            if !value_part.is_empty() {
                let (v, comment) = extract_inline_comment(value_part);
                if !v.is_empty() {
                    tokens.push(Token::Value(v));
                }
                if let Some(c) = comment {
                    tokens.push(Token::Comment(c));
                }
            }
            continue;
        }

        // --- Bare value or unrecognised line ---
        let (v, comment) = extract_inline_comment(content);
        if !v.is_empty() {
            tokens.push(Token::Value(v));
        }
        if let Some(c) = comment {
            tokens.push(Token::Comment(c));
        }
    }

    Ok(tokens)
}

/// Find the index of a structural `:` in `s`.
///
/// Structural means the colon is followed by a space, a tab, or is at the end
/// of the string — and is not inside a double-quoted string.
fn find_structural_colon(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut in_quotes = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quotes = !in_quotes,
            b':' if !in_quotes => {
                let next = bytes.get(i + 1);
                if next.is_none() || *next.unwrap() == b' ' || *next.unwrap() == b'\t' {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Spacing rule: colon ---

    #[test]
    fn colon_space_is_key_separator() {
        let tokens = tokenize("name: Alice\n").unwrap();
        assert!(tokens.contains(&Token::Key("name".into())));
        assert!(tokens.contains(&Token::Value("Alice".into())));
    }

    #[test]
    fn colon_without_space_is_literal_in_value() {
        // "https://example.com" contains `:` not followed by space — it's literal.
        let tokens = tokenize("url: https://example.com\n").unwrap();
        assert!(tokens.contains(&Token::Key("url".into())));
        assert!(tokens.contains(&Token::Value("https://example.com".into())));
    }

    #[test]
    fn colon_at_eol_is_key_separator() {
        // "key:" with nothing after is still a key.
        let tokens = tokenize("key:\n").unwrap();
        assert!(tokens.contains(&Token::Key("key".into())));
    }

    // --- Spacing rule: dash ---

    #[test]
    fn dash_space_is_list_item() {
        let tokens = tokenize("- hello\n").unwrap();
        assert!(tokens.contains(&Token::ListItem));
        assert!(tokens.contains(&Token::Value("hello".into())));
    }

    #[test]
    fn dash_without_space_is_literal_value() {
        // "-not-a-list" is a bare value, not a list item.
        let tokens = tokenize("tag: -draft\n").unwrap();
        assert!(!tokens.contains(&Token::ListItem));
        assert!(tokens.contains(&Token::Value("-draft".into())));
    }

    // --- Spacing rule: hash ---

    #[test]
    fn hash_space_is_comment() {
        let tokens = tokenize("# this is a comment\n").unwrap();
        assert!(tokens.contains(&Token::Comment("this is a comment".into())));
    }

    #[test]
    fn hash_without_space_is_literal() {
        // "#123" inline (no preceding space) should NOT become a Comment token.
        let tokens = tokenize("id: abc#123\n").unwrap();
        assert!(!tokens.iter().any(|t| matches!(t, Token::Comment(_))));
        assert!(tokens.contains(&Token::Value("abc#123".into())));
    }

    #[test]
    fn inline_hash_space_is_trailing_comment() {
        // " # comment" after a value (preceded by space) is structural.
        let tokens = tokenize("key: value # my comment\n").unwrap();
        assert!(tokens.contains(&Token::Key("key".into())));
        assert!(tokens.contains(&Token::Value("value".into())));
        assert!(tokens.contains(&Token::Comment("my comment".into())));
    }

    // --- Literal block ---

    #[test]
    fn literal_block_tokens() {
        let input = "[[[\nhello world\n]]]\n";
        let tokens = tokenize(input).unwrap();
        assert!(tokens.contains(&Token::LiteralBlockOpen));
        assert!(tokens.contains(&Token::Value("hello world\n".into())));
        assert!(tokens.contains(&Token::LiteralBlockClose));
    }

    // --- Key operator-symbol rejection ---

    #[test]
    fn key_starting_with_colon_is_rejected() {
        let result = tokenize(":bad: value\n");
        assert!(result.is_err());
    }
}
