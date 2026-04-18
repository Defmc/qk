use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::Meta;

pub type Result<T> = std::result::Result<T, Error>;

pub mod ast;
pub mod lexer;
pub mod parser;

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum Error {
    #[error("easter egg! This shouldn't be happening")]
    Impossible,

    #[error("it doesn't repeat enough")]
    NotEnoughRepeats,
}

/// returns the metadata and the token index
pub type Token = Meta<usize>;
