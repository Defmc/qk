// Expr = App
// App =
//      Atom+
// Atom =
//      "(" Expr ")"
//      Abs
//      Var
// Abs =
//      "fn" <Ident>+ "=>" Expr
// Var = <Ident>

use std::collections::HashMap;

use miette::Diagnostic;
use thiserror::Error;

use crate::{
    ast::Ast,
    padam::{Token, lexer::Lexer},
};

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("no alternative to parser this snippet")]
    #[diagnostic(code(parser::no_alternative))]
    NoAlternative,

    #[error("there was no enough symbols to repeat the sequence")]
    #[diagnostic(code(parser::no_enough_rep))]
    NoEnoughRep { tks_consumed: usize },
}

impl Error {
    pub fn tokens_consumed(&self) -> Option<usize> {
        match self {
            Self::NoAlternative => None,
            Self::NoEnoughRep { tks_consumed } => Some(*tks_consumed),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
pub type NonTerminals = HashMap<Box<str>, Parser<Ast>>;

pub type CombFn<T> =
    Box<dyn for<'a> Fn(&'a NonTerminals, &'a Lexer, &'a [Token]) -> Result<(T, &'a [Token])>>;

pub struct Parser<T> {
    pub f: CombFn<T>,
}

impl<T: 'static> Parser<T> {
    pub fn new(
        f: impl for<'a> Fn(&'a NonTerminals, &'a Lexer, &'a [Token]) -> Result<(T, &'a [Token])>
        + 'static,
    ) -> Self {
        Self { f: Box::new(f) }
    }

    pub fn parse<'a>(
        &self,
        nt: &'a NonTerminals,
        lex: &'a Lexer,
        tks: &'a [Token],
    ) -> Result<(T, &'a [Token])> {
        (self.f)(nt, lex, tks)
    }

    pub fn seq(seq: Vec<Self>) -> Parser<Vec<T>> {
        Parser::new(move |nt, lex, tks| {
            let mut remaining_tokens = tks;
            let v = seq
                .iter()
                .map(|parser| {
                    parser
                        .parse(nt, lex, remaining_tokens)
                        .inspect(|(_, rem)| remaining_tokens = rem)
                        .map(|(p, _)| p)
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((v, remaining_tokens))
        })
    }

    pub fn or(alternatives: Vec<Self>) -> Self {
        Self::new(move |nt, lex, tks| {
            let mut biggest_err = Error::NoAlternative;
            for alt in &alternatives {
                match alt.parse(nt, lex, tks) {
                    Ok(v) => return Ok(v),
                    Err(e) => {
                        if e.tokens_consumed() > biggest_err.tokens_consumed() {
                            biggest_err = e;
                        }
                    }
                }
            }
            Err(biggest_err)
        })
    }

    pub fn rep(parser: Self, min: usize, max: usize) -> Parser<Vec<T>> {
        Parser::new(move |nt, lex, tks| {
            let mut remaining_tokens = tks;
            let mut v = Vec::new();

            while v.len() < max
                && let Ok((ast, rem)) = parser.parse(nt, lex, remaining_tokens)
            {
                remaining_tokens = rem;
                v.push(ast);
            }

            if v.len() < min {
                return Err(Error::NoEnoughRep {
                    tks_consumed: tks.len() - remaining_tokens.len(),
                });
            }

            return Ok((v, remaining_tokens));
        })
    }

    pub fn plus(parser: Self) -> Parser<Vec<T>> {
        Self::rep(parser, 1, usize::MAX)
    }

    pub fn any(parser: Self) -> Parser<Vec<T>> {
        Self::rep(parser, 0, usize::MAX)
    }

    pub fn option(parser: Self) -> Parser<Vec<T>> {
        Self::rep(parser, 0, 1)
    }

    pub fn external(name: &str) -> Parser<Ast> {
        let name: Box<str> = name.into();
        Parser::new(move |nt, lex, tks| nt[&name].parse(nt, lex, tks))
    }
}
