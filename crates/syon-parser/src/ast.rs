/// A SYON value — the root type of the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Sequence(Vec<Value>),
    Mapping(Vec<(String, Value)>),
}
