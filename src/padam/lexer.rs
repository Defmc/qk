use crate::padam::{Error, Result};
use crate::padam::{ast::Ast, parser::GrammarRule};

pub struct FnToken {
    pub f: Box<dyn Fn(usize, char) -> bool>,
    pub min_amount: usize,

    /// If `true`, the span extends one past the last matching token,
    /// consuming the first non-matching character.
    /// `greedy` WILL NEVER matter for:
    /// - tokens immediately before EOF;
    /// - `min_amount` checking
    pub greedy: bool,
}

impl GrammarRule<str> for FnToken {
    fn parse<'a>(&self, tokens: &'a str) -> Result<(Ast, &'a str)> {
        let end = tokens
            .chars()
            .enumerate()
            .take_while(|(i, c)| (self.f)(*i, *c))
            .count();

        if end >= self.min_amount {
            let end = if self.greedy && tokens.len() > end {
                end + 1
            } else {
                end
            };
            Ok((Ast::Token, &tokens[end..]))
        } else {
            Err(Error::Impossible)
        }
    }

    fn min_length(&self) -> usize {
        self.min_amount
    }
}

pub fn ident() -> FnToken {
    FnToken {
        f: Box::new(|i, c| {
            c == '_'
                || (c >= 'a' && c <= 'z')
                || (c >= 'A' && c <= 'Z')
                || (i > 0 && c >= '0' && c <= '9')
        }),
        greedy: false,
        min_amount: 1,
    }
}

pub fn literal(kw: &str) -> FnToken {
    let chars: Vec<_> = kw.chars().collect();
    FnToken {
        f: Box::new(move |i, c| chars.get(i) == Some(&c)),
        min_amount: kw.len(),
        greedy: false,
    }
}

pub fn char(ch: char) -> FnToken {
    FnToken {
        f: Box::new(move |i, c| i == 0 && ch == c),
        min_amount: 1,
        greedy: false,
    }
}

pub fn comment() -> FnToken {
    FnToken {
        f: Box::new(|i, c| {
            if i == 0 {
                c == '#'
            } else {
                c != '#' && c != '\n'
            }
        }),
        min_amount: 2,
        greedy: true,
    }
}
