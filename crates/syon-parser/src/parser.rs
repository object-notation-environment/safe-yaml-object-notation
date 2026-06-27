use winnow::{
    ascii::{digit1, multispace0},
    combinator::{alt, delimited, opt, preceded, repeat, separated_pair},
    error::ContextError,
    token::{literal, take_while},
    ModalResult, Parser,
};

use crate::ast::Value;

type Input<'s> = &'s str;
type Err = ContextError;

/// Parse a complete SYON document (a single top-level value).
pub fn parse(input: &str) -> Result<Value, String> {
    let input = input.trim();
    parse_value
        .parse(input)
        .map_err(|e| format!("parse error: {e}"))
}

fn parse_value<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    multispace0::<_, Err>.parse_next(input)?;
    alt((
        parse_null,
        parse_bool,
        parse_float,
        parse_integer,
        parse_quoted_string,
        parse_mapping,
        parse_sequence,
        parse_bare_string,
    ))
    .parse_next(input)
}

fn parse_null<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    alt((literal("null"), literal("~")))
        .map(|_| Value::Null)
        .parse_next(input)
}

fn parse_bool<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    alt((
        literal("true").map(|_| Value::Bool(true)),
        literal("false").map(|_| Value::Bool(false)),
    ))
    .parse_next(input)
}

fn parse_integer<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    let sign = opt(literal("-")).parse_next(input)?;
    let digits = digit1.parse_next(input)?;
    let s = format!("{}{}", sign.unwrap_or(""), digits);
    let n: i64 = s.parse().map_err(|_| {
        winnow::error::ErrMode::Backtrack(ContextError::new())
    })?;
    Ok(Value::Int(n))
}

fn parse_float<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    let sign = opt(literal("-")).parse_next(input)?;
    let int_part = digit1.parse_next(input)?;
    let _ = literal(".").parse_next(input)?;
    let frac_part = digit1.parse_next(input)?;
    let s = format!("{}{}.{}", sign.unwrap_or(""), int_part, frac_part);
    let f: f64 = s.parse().map_err(|_| {
        winnow::error::ErrMode::Backtrack(ContextError::new())
    })?;
    Ok(Value::Float(f))
}

fn parse_quoted_string<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    let content = delimited(
        literal("\""),
        take_while(0.., |c| c != '"'),
        literal("\""),
    )
    .parse_next(input)?;
    Ok(Value::Str(content.replace("\\n", "\n").replace("\\\"", "\"")))
}

fn parse_bare_string<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    let s =
        take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
            .parse_next(input)?;
    Ok(Value::Str(s.to_owned()))
}

fn parse_mapping<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    let pairs: Vec<(String, Value)> = delimited(
        literal("{"),
        repeat(
            0..,
            preceded(
                multispace0::<_, Err>,
                separated_pair(
                    parse_key,
                    (multispace0::<_, Err>, literal(":"), multispace0::<_, Err>),
                    parse_value,
                ),
            ),
        ),
        (multispace0::<_, Err>, literal("}")),
    )
    .parse_next(input)?;
    Ok(Value::Mapping(pairs))
}

fn parse_key<'s>(input: &mut Input<'s>) -> ModalResult<String, Err> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-')
        .map(|s: &str| s.to_owned())
        .parse_next(input)
}

fn parse_sequence<'s>(input: &mut Input<'s>) -> ModalResult<Value, Err> {
    let items: Vec<Value> = delimited(
        literal("["),
        repeat(
            0..,
            preceded(
                (multispace0::<_, Err>, opt(literal(","))),
                parse_value,
            ),
        ),
        (multispace0::<_, Err>, literal("]")),
    )
    .parse_next(input)?;
    Ok(Value::Sequence(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;

    #[test]
    fn parse_null_value() {
        assert_eq!(parse("null").unwrap(), Value::Null);
        assert_eq!(parse("~").unwrap(), Value::Null);
    }

    #[test]
    fn parse_bool_values() {
        assert_eq!(parse("true").unwrap(), Value::Bool(true));
        assert_eq!(parse("false").unwrap(), Value::Bool(false));
    }

    #[test]
    fn parse_integer_value() {
        assert_eq!(parse("42").unwrap(), Value::Int(42));
        assert_eq!(parse("-7").unwrap(), Value::Int(-7));
    }

    #[test]
    fn parse_float_value() {
        assert_eq!(parse("3.14").unwrap(), Value::Float(3.14));
    }

    #[test]
    fn parse_quoted_string_value() {
        assert_eq!(
            parse("\"hello world\"").unwrap(),
            Value::Str("hello world".into())
        );
    }

    #[test]
    fn parse_inline_sequence() {
        assert_eq!(
            parse("[1 2 3]").unwrap(),
            Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn parse_inline_mapping() {
        let v = parse("{name: Alice}").unwrap();
        assert_eq!(
            v,
            Value::Mapping(vec![("name".into(), Value::Str("Alice".into()))])
        );
    }

    #[test]
    fn parse_bare_string_value() {
        assert_eq!(parse("hello").unwrap(), Value::Str("hello".into()));
    }
}
