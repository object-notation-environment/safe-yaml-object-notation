/// Lexer token types for SYON.
///
/// SYON is a YAML-inspired safe subset: no anchors, no aliases, no arbitrary
/// tags, no multi-document streams.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A bare key or unquoted string value.
    Ident(String),
    /// A double-quoted string.
    QuotedStr(String),
    /// An integer literal.
    Integer(i64),
    /// A floating-point literal.
    Float(f64),
    /// `true` or `false`.
    Bool(bool),
    /// `null` or `~`.
    Null,
    /// `:` (mapping separator).
    Colon,
    /// `-` at the start of a line (sequence item marker).
    Dash,
    /// A newline / line-end.
    Newline,
    /// Indentation change (number of leading spaces on a new non-empty line).
    Indent(usize),
}

/// Tokenise a SYON source string into a flat list of tokens.
///
/// This is a simple character-by-character pass; the full context-sensitive
/// indent tracking is handled by the parser.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            tokens.push(Token::Newline);
            continue;
        }

        // Count leading spaces for indent tracking.
        let indent = line.len() - line.trim_start().len();
        tokens.push(Token::Indent(indent));

        let rest = trimmed.trim_start();

        // Sequence item.
        if let Some(after_dash) = rest.strip_prefix("- ") {
            tokens.push(Token::Dash);
            tokenize_value(after_dash.trim(), &mut tokens)?;
        } else if let Some(colon_pos) = find_key_colon(rest) {
            let key = rest[..colon_pos].trim().to_owned();
            tokens.push(Token::Ident(key));
            tokens.push(Token::Colon);
            let value_part = rest[colon_pos + 1..].trim();
            if !value_part.is_empty() {
                tokenize_value(value_part, &mut tokens)?;
            }
        } else {
            tokenize_value(rest, &mut tokens)?;
        }

        tokens.push(Token::Newline);
    }

    Ok(tokens)
}

/// Find the position of `:` that acts as a mapping key separator (not inside a
/// quoted string, and followed by a space or end-of-line).
fn find_key_colon(s: &str) -> Option<usize> {
    let mut in_quotes = false;
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' => in_quotes = !in_quotes,
            ':' if !in_quotes => {
                let next = chars.get(i + 1);
                if next.is_none() || next == Some(&' ') {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Lex a single scalar value token and push it onto `out`.
fn tokenize_value(s: &str, out: &mut Vec<Token>) -> Result<(), String> {
    if s == "null" || s == "~" {
        out.push(Token::Null);
    } else if s == "true" {
        out.push(Token::Bool(true));
    } else if s == "false" {
        out.push(Token::Bool(false));
    } else if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        out.push(Token::QuotedStr(inner.replace("\\\"", "\"").replace("\\n", "\n")));
    } else if let Ok(i) = s.parse::<i64>() {
        out.push(Token::Integer(i));
    } else if let Ok(f) = s.parse::<f64>() {
        out.push(Token::Float(f));
    } else {
        out.push(Token::Ident(s.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_mapping() {
        let src = "name: Alice\nage: 30\n";
        let tokens = tokenize(src).unwrap();
        assert!(tokens.contains(&Token::Ident("name".into())));
        assert!(tokens.contains(&Token::Colon));
        assert!(tokens.contains(&Token::Ident("Alice".into())));
        assert!(tokens.contains(&Token::Integer(30)));
    }

    #[test]
    fn tokenize_sequence_items() {
        let src = "- foo\n- bar\n";
        let tokens = tokenize(src).unwrap();
        assert_eq!(tokens.iter().filter(|t| **t == Token::Dash).count(), 2);
        assert!(tokens.contains(&Token::Ident("foo".into())));
        assert!(tokens.contains(&Token::Ident("bar".into())));
    }

    #[test]
    fn tokenize_null_and_booleans() {
        let src = "a: null\nb: true\nc: false\n";
        let tokens = tokenize(src).unwrap();
        assert!(tokens.contains(&Token::Null));
        assert!(tokens.contains(&Token::Bool(true)));
        assert!(tokens.contains(&Token::Bool(false)));
    }

    #[test]
    fn tokenize_quoted_string() {
        let src = "msg: \"hello world\"\n";
        let tokens = tokenize(src).unwrap();
        assert!(tokens.contains(&Token::QuotedStr("hello world".into())));
    }

    #[test]
    fn tokenize_float() {
        let src = "ratio: 3.14\n";
        let tokens = tokenize(src).unwrap();
        assert!(tokens.contains(&Token::Float(3.14)));
    }
}
