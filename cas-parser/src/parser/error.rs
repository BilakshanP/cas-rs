use crate::tokenizer::TokenKind;
use std::ops::Range;

/// The kind of error that occurred.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// The end of the source code was reached unexpectedly.
    Eof,

    /// An unexpected token was encountered.
    UnexpectedToken {
        /// The token(s) that were expected.
        expected: &'static [TokenKind],

        /// The token that was found.
        found: TokenKind,
    },

    /// A non-fatal error. If this error is encountered, the parser should try parsing other
    /// branches.
    NonFatal,
}

/// A general parsing error.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    /// The region of the source code that this error originated from.
    pub span: Range<usize>,

    /// The kind of error that occurred.
    pub kind: ErrorKind,
}

impl Error {
    /// Creates a new error with the given span and kind.
    pub fn new(span: Range<usize>, kind: ErrorKind) -> Self {
        Self { span, kind }
    }
}
