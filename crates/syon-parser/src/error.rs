use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum SyonError {
    /// A YAML construct that SYON explicitly forbids.
    Forbidden(String),
    /// A low-level syntax / parse error.
    Syntax(String),
}

impl fmt::Display for SyonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyonError::Forbidden(msg) => write!(f, "forbidden: {msg}"),
            SyonError::Syntax(msg) => write!(f, "syntax error: {msg}"),
        }
    }
}

impl std::error::Error for SyonError {}
