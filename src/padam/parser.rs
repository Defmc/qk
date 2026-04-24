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

pub type Rule = Box<dyn GrammarRule<[Token]>>;

use crate::padam::{Error, Result, Token};

pub struct Parser {
    pub grammar: Rule,
}

pub trait GrammarRule<T: ?Sized> {
    fn parse<'a>(&self, tokens: &'a T) -> Result<(Ast, &'a T)>;
}

pub type Result<T> = std::result::Result<T, Error>;
pub type NonTerminals = HashMap<Box<str>, Box<dyn Parser<Ast>>>;

pub trait Parser<T> {
    fn parse<'a>(
        &self,
        nt: &'a NonTerminals,
        lex: &'a Lexer,
        tks: &'a [Token],
    ) -> Result<(T, &'a [Token])>;
}

pub struct Seq<T> {
    seq: Vec<Box<dyn Parser<T>>>,
}

impl<T> Seq<T> {
    pub fn new(seq: Vec<Box<dyn Parser<T>>>) -> Self {
        Self { seq }
    }
}

impl<T> Parser<Vec<T>> for Seq<T> {
    fn parse<'a>(
        &self,
        nt: &'a NonTerminals,
        lex: &'a Lexer,
        tks: &'a [Token],
    ) -> Result<(Vec<T>, &'a [Token])> {
        let mut remaining_tokens = tks;
        let v = self
            .seq
            .iter()
            .map(|parser| {
                parser
                    .parse(nt, lex, remaining_tokens)
                    .inspect(|(_, rem)| remaining_tokens = rem)
                    .map(|(p, _)| p)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok((v, remaining_tokens))
    }
}

pub struct Or<T> {
    alternatives: Vec<Box<dyn Parser<T>>>,
}

impl<T> Or<T> {
    pub fn new(alternatives: Vec<Box<dyn Parser<T>>>) -> Self {
        Self { alternatives }
    }
}

impl<T> Parser<T> for Or<T> {
    fn parse<'a>(
        &self,
        nt: &'a NonTerminals,
        lex: &'a Lexer,
        tks: &'a [Token],
    ) -> Result<(T, &'a [Token])> {
        let mut biggest_err = Error::NoAlternative;
        for alt in &self.alternatives {
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
    }
}

pub struct Rep<T> {
    parser: Box<dyn Parser<T>>,
    min: usize,
    max: usize,
}

impl<T> Rep<T> {
    pub fn new(parser: Box<dyn Parser<T>>, min: usize, max: usize) -> Self {
        Self { parser, min, max }
    }

    // T+
    pub fn plus(parser: Box<dyn Parser<T>>) -> Self {
        Self::new(parser, 1, usize::MAX)
    }

    // T*
    pub fn any(parser: Box<dyn Parser<T>>) -> Self {
        Self::new(parser, 0, usize::MAX)
    }

    // T?
    pub fn option(parser: Box<dyn Parser<T>>) -> Self {
        Self::new(parser, 0, 1)
    }
}

impl<T> Parser<Vec<T>> for Rep<T> {
    fn parse<'a>(
        &self,
        nt: &'a NonTerminals,
        lex: &'a Lexer,
        tks: &'a [Token],
    ) -> Result<(Vec<T>, &'a [Token])> {
        let mut remaining_tokens = tks;
        let mut v = Vec::new();

        while v.len() < self.max
            && let Ok((ast, rem)) = self.parser.parse(nt, lex, remaining_tokens)
        {
            remaining_tokens = rem;
            v.push(ast);
        }

        if v.len() < self.min {
            return Err(Error::NoEnoughRep {
                tks_consumed: tks.len() - remaining_tokens.len(),
            });
        }

        return Ok((v, remaining_tokens));
    }
}
