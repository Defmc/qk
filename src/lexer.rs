use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Meta<T> {
    pub item: T,
    pub at: SourceSpan,
}

impl<T> Meta<T> {
    pub fn from_code<'a>(&self, src: &'a str) -> &'a str {
        from_code(self.at, src)
    }
}

pub trait Trace
where
    Self: Sized,
{
    fn at(self, span: SourceSpan) -> Box<Meta<Self>> {
        Meta {
            item: self,
            at: span,
        }
        .into()
    }

    fn generated(self) -> Box<Meta<Self>> {
        const GENERATED_RANGE: std::ops::RangeInclusive<usize> = 0..=0;
        self.at(GENERATED_RANGE.into())
    }
}

impl<T> Trace for T {}

pub fn from_code(s: SourceSpan, src: &str) -> &str {
    &src[s.offset()..s.offset() + s.len()]
}

pub fn over(l: SourceSpan, r: SourceSpan) -> SourceSpan {
    SourceSpan::new(l.offset().into(), r.offset() + r.len() - l.offset())
}

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
