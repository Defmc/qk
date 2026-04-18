use crate::lexer::Meta;
use crate::padam::{Error, Result, Token};
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
            .char_indices()
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
        min_amount: kw.chars().count(),
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

use crate::padam::ast::AstNode;

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
        Self {
            tokenizers: tokenizers.collect(),
        }
    }

    pub fn push(&mut self, tokenizer: Tokenizer) {
        self.tokenizers.push(tokenizer);
    }

    pub fn lex(&self, src: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut start = 0;
        while start < src.len() {
            let offset = &src[start..];
            let (i, (_, res)) = self.single_lex(offset)?;
            let span = (start, offset.len() - res.len()).into();
            start += offset.len() - res.len();
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
            Tokenizer::new("FnKw", literal("fn")),
            Tokenizer::new("FnImpl", literal("=>")),
            Tokenizer::new("OpenParen", char('(')),
            Tokenizer::new("CloseParen", char(')')),
            Tokenizer::new("Eol", char('\n')),
            Tokenizer::comment(char(' ')),
            Tokenizer::comment(char('\t')),
            Tokenizer::new("Ident", ident()),
            Tokenizer::comment(comment()),
            Tokenizer::new("Assign", char('=')),
        ]
        .into_iter();
        Self::new(tokenizers)
    }
}

// #[cfg(test)]
// pub mod tests {
//     use crate::padam::lexer::Lexer;
//
//     pub fn expected(lexer: &Lexer, source: &str, values: &[&str]) {
//         let mut s = String::new();
//         let lexs = lexer.lex(source).unwrap();
//         print!("lexemes: ");
//         for (i, (l, r)) in lexs.iter().zip(values.iter()).enumerate() {
//             let l_ty = lexer.get_type(l.item);
//             print!("{:?} ({l_ty}) ", l.from_code(source));
//             if l_ty != *r {
//                 s.push_str(&format!("\t{i}. {:?} ({l_ty}) != {r}", l.from_code(source)));
//             }
//         }
//         assert_eq!(
//             lexs.len(),
//             values.len(),
//             "token count mismatch: got {}, expected {}",
//             lexs.len(),
//             values.len()
//         );
//         if !s.is_empty() {
//             panic!("lexer mismatch:\n{s}");
//         }
//     }
// }
