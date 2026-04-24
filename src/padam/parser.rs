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

pub struct Or {
    pub variants: Vec<Rule>,
}

impl GrammarRule<[Token]> for Or {
    fn parse<'a>(&self, tokens: &'a [Token]) -> Result<(Ast, &'a [Token])> {
        let mut last_attempt = Err(Error::Impossible);
        for var in &self.variants {
            last_attempt = var.parse(tokens);
            if last_attempt.is_ok() {
                break;
            }
        }
        last_attempt
    }
}

pub struct Any {
    pub item: Rule,
    pub min_amount: usize,
    pub redex: Box<dyn Fn(Vec<Ast>) -> Ast>,
}

impl GrammarRule<[Token]> for Any {
    fn parse<'a>(&self, tokens: &'a [Token]) -> Result<(Ast, &'a [Token])> {
        let mut tokens = tokens;
        let mut build = Vec::with_capacity(self.min_amount);
        let mut last = self.item.parse(tokens);
        while let Ok((last_ast, last_tks)) = last {
            build.push(last_ast);
            tokens = &tokens[last_tks.len()..];
            last = self.item.parse(tokens);
        }
        if build.len() >= self.min_amount {
            Ok(((self.redex)(build), tokens))
        } else {
            Err(Error::NotEnoughRepeats)
        }
    }
}
