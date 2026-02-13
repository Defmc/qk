use logos::Logos;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Meta<T> {
    pub item: T,
    pub at: SourceSpan,
}

pub fn over(l: SourceSpan, r: SourceSpan) -> SourceSpan {
    SourceSpan::new(l.offset().into(), r.offset() + r.len() - l.offset())
}

pub type Token = Meta<TkTy>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Diagnostic, Default, Clone, PartialEq)]
pub enum Error {
    #[error("invalid char sequence")]
    #[diagnostic(
        code(lexer::invalid_char_seq),
        help("these chars doesn't belong to this code. Haven't you mistyped?")
    )]
    InvalidCharSeq {
        #[label("here")]
        at: SourceSpan,
    },

    #[default]
    #[error("other error")]
    #[diagnostic(code(lexer::other_error), help("this shouldn't happen. contact me"))]
    Other,
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+", error = Error)]
pub enum TkTy {
    #[token("\\")]
    #[token("λ")]
    #[token("fn")]
    Function,

    #[token("=>")]
    #[token(".")]
    Abstraction,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex("[a-zA-Z]+")]
    Variable,
}

impl TkTy {
    pub fn processed(s: &str) -> impl Iterator<Item = Result<Meta<TkTy>>> {
        TkTy::lexer(s).spanned().map(|(tk, s)| {
            let at = SourceSpan::new(s.start.into(), s.end - s.start);
            tk.map_or_else(
                // TODO: use `_e` wiser.
                |_e| Err(Error::InvalidCharSeq { at }),
                |tk| Ok(Meta { item: tk, at }),
            )
        })
    }
}
