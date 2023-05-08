pub mod binary;
pub mod error;
pub mod expr;
pub mod literal;
pub mod token;
pub mod unary;

use error::{Error, ErrorKind};
use super::tokenizer::{tokenize_complete, Token};
use std::ops::Range;

/// A high-level parser for the language. This is the type to use to parse an arbitrary piece of
/// code into an abstract syntax tree.
#[derive(Debug, Clone)]
pub struct Parser<'source> {
    /// The tokens that this parser is currently parsing.
    tokens: Box<[Token<'source>]>,

    /// The index of the **next** token to be parsed.
    cursor: usize,
}

impl<'source> Parser<'source> {
    /// Create a new parser for the given source.
    pub fn new(source: &'source str) -> Self {
        Self {
            tokens: tokenize_complete(source).unwrap(),
            cursor: 0,
        }
    }

    /// Creates an error that points at the current token, or the end of the source code if the
    /// cursor is at the end of the stream.
    pub fn error(&self, kind: ErrorKind) -> Error {
        Error::new(self.span(), kind)
    }

    /// Creates an [`ErrorKind::NonFatal`] error that points at the current token.
    pub fn non_fatal(&self) -> Error {
        Error::new(self.span(), ErrorKind::NonFatal)
    }

    /// Returns a span pointing at the end of the source code.
    pub fn eof_span(&self) -> Range<usize> {
        self.tokens.last().map_or(0..0, |token| token.span.end..token.span.end)
    }

    /// Returns the span of the current token, or the end of the source code if the cursor is at
    /// the end of the stream.
    pub fn span(&self) -> Range<usize> {
        self.tokens
            .get(self.cursor)
            .map_or(self.eof_span(), |token| token.span.clone())
    }

    /// Returns the previous token. The cursor is not moved. Returns [`None`] if the cursor is at
    /// the beginning of the stream.
    pub fn prev_token(&self) -> Option<&Token<'source>> {
        self.tokens.get(self.cursor.checked_sub(1)?)
    }

    /// Returns the next token to be parsed, then advances the cursor. Whitespace tokens are
    /// skipped.
    ///
    /// Returns an EOF error if there are no more tokens.
    pub fn next_token(&mut self) -> Result<Token<'source>, Error> {
        while self.cursor < self.tokens.len() {
            let token = &self.tokens[self.cursor];
            self.cursor += 1;
            if token.is_whitespace() {
                continue;
            } else {
                // cloning is cheap: only Range<_> is cloned
                return Ok(token.clone());
            }
        }

        Err(self.error(ErrorKind::UnexpectedEof))
    }

    /// Speculatively parses a value from the given stream of tokens. This function can be used
    /// in the [`Parse::parse`] implementation of a type with the given [`Parser`], as it will
    /// automatically backtrack the cursor position if parsing fails.
    ///
    /// If parsing is successful, the stream is advanced past the consumed tokens and the parsed
    /// value is returned. Otherwise, the stream is left unchanged and an error is returned.
    pub fn try_parse<T: Parse>(&mut self) -> Result<T, Error> {
        self.try_parse_with_fn(T::parse)
    }

    /// Speculatively parses a value from the given stream of tokens, using a custom parsing
    /// function to parse the value. This function can be used in the [`Parse::parse`]
    /// implementation of a type with the given [`Parser`], as it will automatically backtrack the
    /// cursor position if parsing fails.
    ///
    /// If parsing is successful, the stream is advanced past the consumed tokens and the parsed
    /// value is returned. Otherwise, the stream is left unchanged and an error is returned.
    pub fn try_parse_with_fn<T, F>(&mut self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&mut Parser) -> Result<T, Error>,
    {
        let start = self.cursor;
        match f(self) {
            Ok(value) => Ok(value),
            err => {
                self.cursor = start;
                err
            },
        }
    }

    /// Speculatively parses a value from the given stream of tokens, with a validation predicate.
    /// The value must parse successfully, **and** the predicate must return [`Ok`] for this
    /// function to return successfully.
    ///
    /// If parsing is successful, the stream is advanced past the consumed tokens and the parsed
    /// value is returned. Otherwise, the stream is left unchanged and an error is returned.
    pub fn try_parse_then<T: Parse, F>(&mut self, predicate: F) -> Result<T, Error>
    where
        F: FnOnce(&T, &Parser) -> Result<(), Error>,
    {
        let start = self.cursor;

        // closure workaround allows us to use `?` in the closure
        let compute = || {
            let value = T::parse(self)?;
            predicate(&value, self)?;
            Ok(value)
        };

        match compute() {
            Ok(value) => Ok(value),
            err => {
                self.cursor = start;
                err
            },
        }
    }

    /// Attempts to parse a value from the given stream of tokens. All the tokens must be consumed
    /// by the parser; if not, an error is returned.
    pub fn try_parse_full<T: Parse>(&mut self) -> Result<T, Error> {
        let value = T::parse(self)?;
        if self.cursor == self.tokens.len() {
            Ok(value)
        } else {
            Err(self.error(ErrorKind::ExpectedEof))
        }
    }
}

/// Any type that can be parsed from a source of tokens.
pub trait Parse: Sized {
    /// Parses a value from the given stream of tokens, advancing the stream past the consumed
    /// tokens if parsing is successful.
    ///
    /// This function should be used by consumers of the library.
    fn parse(input: &mut Parser) -> Result<Self, Error>;
}

/// The associativity of a binary or unary operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Associativity {
    /// The binary / unary operation is left-associative.
    ///
    /// For binary operations, this means `a op b op c` is evaluated as `(a op b) op c`. For unary
    /// operations, this means `a op op` is evaluated as `(a op) op` (the operators appear to the
    /// right of the operand).
    Left,

    /// The binary / unary operation is right-associative.
    ///
    /// For binary operations, this means `a op b op c` is evaluated as `a op (b op c)`. For unary
    /// operations, this means `op op a` is evaluated as `op (op a)` (the operators appear to the
    /// left of the operand).
    Right,
}

/// The precedence of an operation, in order from lowest precedence (evaluated last) to highest
/// precedence (evaluated first).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Precedence {
    /// Any precedence.
    Any,

    /// Precedence of addition (`+`) and subtraction (`-`), which separate terms.
    Term,

    /// Precedence of multiplication (`*`), division (`/`), and modulo (`%`), which separate
    /// factors.
    Factor,

    /// Precedence of exponentiation (`^`).
    Exp,

    /// Precedence of factorial (`!`).
    Factorial,

    /// Precedence of unary subtraction (`-`).
    Neg,

    /// Precedence of logical not (`not`).
    Not,
}

impl PartialOrd for Precedence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let left = *self as u8;
        let right = *other as u8;
        left.partial_cmp(&right)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use super::*;

    use binary::Binary;
    use expr::Expr;
    use literal::{Literal, LitNum};
    use token::op::{BinOp, UnaryOp};
    use unary::Unary;

    #[test]
    fn literal_int() {
        let mut parser = Parser::new("16");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Literal(Literal::Number(LitNum {
            value: 16.0,
            span: 0..2,
        })));
    }

    #[test]
    fn literal_float() {
        let mut parser = Parser::new("3.14");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Literal(Literal::Number(LitNum {
            value: 3.14,
            span: 0..4,
        })));
    }

    #[test]
    fn unary_left_associativity() {
        let mut parser = Parser::new("3!!");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Unary(Box::new(Unary {
            operand: Expr::Unary(Box::new(Unary {
                operand: Expr::Literal(Literal::Number(LitNum {
                    value: 3.0,
                    span: 0..1,
                })),
                op: UnaryOp::Factorial,
                span: 0..2,
            })),
            op: UnaryOp::Factorial,
            span: 0..3,
        })));
    }

    #[test]
    fn unary_right_associativity() {
        let mut parser = Parser::new("not not --3");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Unary(Box::new(Unary {
            operand: Expr::Unary(Box::new(Unary {
                operand: Expr::Unary(Box::new(Unary {
                    operand: Expr::Unary(Box::new(Unary {
                        operand: Expr::Literal(Literal::Number(LitNum {
                            value: 3.0,
                            span: 10..11,
                        })),
                        op: UnaryOp::Neg,
                        span: 9..11,
                    })),
                    op: UnaryOp::Neg,
                    span: 8..11,
                })),
                op: UnaryOp::Not,
                span: 4..11,
            })),
            op: UnaryOp::Not,
            span: 0..11,
        })));
    }

    #[test]
    fn binary_left_associativity() {
        let mut parser = Parser::new("3 * 4 * 5");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Binary(Box::new(Binary {
                lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                    value: 3.0,
                    span: 0..1,
                }))),
                op: BinOp::Mul,
                rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                    value: 4.0,
                    span: 4..5,
                }))),
                span: 0..5,
            }))),
            op: BinOp::Mul,
            rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 5.0,
                span: 8..9,
            }))),
            span: 0..9,
        })));
    }

    #[test]
    fn binary_left_associativity_mix_precedence() {
        let mut parser = Parser::new("3 + 4 * 5 + 6");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Binary(Box::new(Binary {
                lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                    value: 3.0,
                    span: 0..1,
                }))),
                op: BinOp::Add,
                rhs: Box::new(Expr::Binary(Box::new(Binary {
                    lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                        value: 4.0,
                        span: 4..5,
                    }))),
                    op: BinOp::Mul,
                    rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                        value: 5.0,
                        span: 8..9,
                    }))),
                    span: 4..9,
                }))),
                span: 0..9,
            }))),
            op: BinOp::Add,
            rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 6.0,
                span: 12..13,
            }))),
            span: 0..13,
        })));
    }

    #[test]
    fn binary_right_associativity() {
        let mut parser = Parser::new("1 ^ 2 ^ 3");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 1.0,
                span: 0..1,
            }))),
            op: BinOp::Exp,
            rhs: Box::new(Expr::Binary(Box::new(Binary {
                lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                    value: 2.0,
                    span: 4..5,
                }))),
                op: BinOp::Exp,
                rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                    value: 3.0,
                    span: 8..9,
                }))),
                span: 4..9,
            }))),
            span: 0..9,
        })));
    }

    #[test]
    fn binary_complicated() {
        let mut parser = Parser::new("1 + 2 * 3 - 4 / 5 ^ 6");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        // 2 * 3
        let mul = Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 2.0,
                span: 4..5,
            }))),
            op: BinOp::Mul,
            rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 3.0,
                span: 8..9,
            }))),
            span: 4..9,
        }));

        // 1 + 2 * 3
        let add = Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 1.0,
                span: 0..1,
            }))),
            op: BinOp::Add,
            rhs: Box::new(mul),
            span: 0..9,
        }));

        // 5 ^ 6
        let exp = Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 5.0,
                span: 16..17,
            }))),
            op: BinOp::Exp,
            rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 6.0,
                span: 20..21,
            }))),
            span: 16..21,
        }));

        // 4 / 5 ^ 6
        let div = Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 4.0,
                span: 12..13,
            }))),
            op: BinOp::Div,
            rhs: Box::new(exp),
            span: 12..21,
        }));

        // 1 + 2 * 3 - 4 / 5 ^ 6
        let sub = Expr::Binary(Box::new(Binary {
            lhs: Box::new(add),
            op: BinOp::Sub,
            rhs: Box::new(div),
            span: 0..21,
        }));

        assert_eq!(expr, sub);
    }

    #[test]
    fn binary_and_unary() {
        let mut parser = Parser::new("-1 ^ -2 * 3");
        let expr = parser.try_parse_full::<Expr>().unwrap();

        assert_eq!(expr, Expr::Binary(Box::new(Binary {
            lhs: Box::new(Expr::Unary(Box::new(Unary {
                operand: Expr::Binary(Box::new(Binary {
                    lhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                        value: 1.0,
                        span: 1..2,
                    }))),
                    op: BinOp::Exp,
                    rhs: Box::new(Expr::Unary(Box::new(Unary {
                        operand: Expr::Literal(Literal::Number(LitNum {
                            value: 2.0,
                            span: 6..7,
                        })),
                        op: UnaryOp::Neg,
                        span: 5..7,
                    }))),
                    span: 1..7,
                })),
                op: UnaryOp::Neg,
                span: 0..7,
            }))),
            op: BinOp::Mul,
            rhs: Box::new(Expr::Literal(Literal::Number(LitNum {
                value: 3.0,
                span: 10..11,
            }))),
            span: 0..11,
        })));
    }
}
