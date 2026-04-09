use miette::Diagnostic;
use thiserror::Error;

use crate::{
    lexer::Meta,
    padam::{ast::AstNode, parser::GrammarRule},
};

pub type Result<T> = std::result::Result<T, Error>;

pub mod ast;
pub mod lexer;
pub mod parser;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("easter egg! This shouldn't be happening")]
    Impossible,

    #[error("it doesn't repeat enough")]
    NotEnoughRepeats,
}

/// returns the metadata and the token index
pub type Token = Meta<usize>;

pub struct Tokenizer {
    pub name: Box<str>,
    pub toker: Box<dyn GrammarRule<str>>,
    pub ignore: bool,
}

impl Tokenizer {
    pub fn new<T: GrammarRule<str> + 'static>(name: &str, gr: T) -> Self {
        Self {
            name: name.into(),
            toker: Box::new(gr),
            ignore: false,
        }
    }

    pub fn comment<T: GrammarRule<str> + 'static>(gr: T) -> Self {
        Self {
            name: Box::default(),
            toker: Box::new(gr),
            ignore: true,
        }
    }
}

pub struct Lexer {
    tokenizers: Vec<Tokenizer>,
}

impl Lexer {
    pub fn new(tokenizers: impl Iterator<Item = Tokenizer>) -> Self {
        let mut s = Self {
            tokenizers: tokenizers.collect(),
        };
        s.sort();
        s
    }

    fn sort(&mut self) {
        self.tokenizers
            .sort_by_key(|t| std::cmp::Reverse(t.toker.min_length()));
    }

    pub fn push(&mut self, tokenizer: Tokenizer) {
        self.tokenizers.push(tokenizer);
        self.sort();
    }

    pub fn span_from_origin(src: &str, offset: &str, res: &str) -> miette::SourceSpan {
        let end = src.len() - res.len();
        let start = src.len() - offset.len();
        (start, end).into()
    }

    pub fn lex(&self, src: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut start = 0;
        while start < src.len() {
            let offset = &src[start..];
            let (i, (_, res)) = self.single_lex(offset)?;
            let span = Self::span_from_origin(src, offset, res);
            start = span.offset() + span.len();
            if !self.tokenizers[i].ignore {
                tokens.push(Meta { item: i, at: span });
            }
        }
        Ok(tokens)
    }

    pub fn single_lex<'a>(&'a self, src: &'a str) -> Result<(usize, (AstNode, &'a str))> {
        let mut atom = Err(Error::Impossible);
        self.tokenizers
            .iter()
            .enumerate()
            .for_each(|(i, tokenizer)| match tokenizer.toker.parse(src) {
                Ok((tok, s)) => {
                    if s.len()
                        < atom.as_ref().map_or_else(
                            |_| usize::MAX,
                            |(_, (_, old_s)): &(usize, (AstNode, &str))| old_s.len(),
                        )
                    {
                        atom = Ok((i, (tok, s)))
                    }
                }
                Err(_) => (),
            });
        atom
    }

    pub fn get_type(&self, toker_idx: usize) -> &str {
        self.tokenizers[toker_idx].name.as_ref()
    }
}

impl Default for Lexer {
    fn default() -> Self {
        let tokenizers = [
            Tokenizer::new("Ident", lexer::ident()),
            Tokenizer::new("FnKw", lexer::literal("fn")),
            Tokenizer::new("FnImpl", lexer::literal("=>")),
            Tokenizer::new("OpenParen", lexer::char('(')),
            Tokenizer::new("CloseParen", lexer::char(')')),
            Tokenizer::comment(lexer::comment()),
        ]
        .into_iter();
        Self::new(tokenizers)
    }
}
