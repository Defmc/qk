use logos::Logos;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Meta<T> {
    pub item: T,
    pub at: SourceSpan,
}

impl<T> Meta<T> {
    pub fn from_code<'a>(&self, src: &'a str) -> &'a str {
        from_code(self.at, src)
    }
}

pub fn from_code(s: SourceSpan, src: &str) -> &str {
    &src[s.offset()..s.offset() + s.len()]
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
            let at = (s.start..=s.end).into();
            tk.map_or_else(
                // TODO: use `_e` wiser.
                |_e| Err(Error::InvalidCharSeq { at }),
                |tk| Ok(Meta { item: tk, at }),
            )
        })
    }
}
