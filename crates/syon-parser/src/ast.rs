/// A comment node — first-class in the SYON AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub text: String,
}

/// A key–value pair inside a mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingEntry {
    pub key: String,
    pub value: Value,
    /// Comment lines that appear immediately before this key on their own line.
    pub leading_comments: Vec<Comment>,
    /// A `# …` comment on the same line as the key or value.
    pub trailing_comment: Option<Comment>,
}

/// A SYON mapping (ordered).
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    pub entries: Vec<MappingEntry>,
}

/// A SYON sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub items: Vec<Value>,
}

/// A SYON value node.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Mapping(Mapping),
    Sequence(Sequence),
    /// All scalars are strings at the parse boundary — no implicit typing.
    Scalar(String),
    /// Verbatim content from a `[[[` … `]]]` literal-block escape hatch.
    LiteralBlock(String),
}

/// The root of a parsed SYON document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub body: Value,
    pub trailing_comments: Vec<Comment>,
}
