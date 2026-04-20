use crate::lexer::Meta;
use crate::padam::{Error, Result, Token};

/// A raw component from the source-code
pub trait Lexeme {
    /// returns the respective lexeme for this token
    fn parse<'a>(&self, tokens: &'a str) -> Result<&'a str>;
}

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

impl Lexeme for FnToken {
    fn parse<'a>(&self, tokens: &'a str) -> Result<&'a str> {
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
            Ok(&tokens[..end])
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

pub fn single_char(ch: char) -> FnToken {
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

pub struct Tokenizer {
    pub name: Box<str>,
    pub toker: Box<dyn Lexeme>,
    pub ignore: bool,
}

impl Tokenizer {
    pub fn new<T: Lexeme + 'static>(name: &str, gr: T) -> Self {
        Self {
            name: name.into(),
            toker: Box::new(gr),
            ignore: false,
        }
    }

    pub fn ignore<T: Lexeme + 'static>(gr: T) -> Self {
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
            let (i, lexeme) = self.single_lex(offset)?;
            let span = (start, lexeme.len()).into();
            start += lexeme.len();
            if !self.tokenizers[i].ignore {
                tokens.push(Meta { item: i, at: span });
            }
        }
        Ok(tokens)
    }

    pub fn single_lex<'a>(&'a self, src: &'a str) -> Result<(usize, &'a str)> {
        self.tokenizers
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.toker.parse(src).ok().map(|tk| (i, tk)))
            .max_by_key(|(_, span)| span.chars().count())
            .map_or_else(|| Err(Error::Impossible), Ok)
    }

    pub fn get_type(&self, toker_idx: usize) -> &str {
        self.tokenizers[toker_idx].name.as_ref()
    }
}

const FN_KW_TY: &str = "FnKw";
const FN_IMPL_TY: &str = "FnImpl";
const OPEN_PAREN_TY: &str = "OpenParen";
const CLOSE_PAREN_TY: &str = "CloseParen";
const EOL_TY: &str = "Eol";
const IDENT_TY: &str = "Ident";
const ASSIGN_TY: &str = "Assign";

impl Default for Lexer {
    fn default() -> Self {
        let tokenizers = [
            Tokenizer::new(FN_KW_TY, literal("fn")),
            Tokenizer::new(FN_IMPL_TY, literal("=>")),
            Tokenizer::new(OPEN_PAREN_TY, single_char('(')),
            Tokenizer::new(CLOSE_PAREN_TY, single_char(')')),
            Tokenizer::new(EOL_TY, single_char('\n')),
            Tokenizer::ignore(single_char(' ')),
            Tokenizer::ignore(single_char('\t')),
            Tokenizer::new(IDENT_TY, ident()),
            Tokenizer::ignore(comment()),
            Tokenizer::new(ASSIGN_TY, single_char('=')),
        ]
        .into_iter();
        Self::new(tokenizers)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::padam::lexer::Lexer;

    pub fn expected(lexer: &Lexer, source: &str, values: &[&str]) {
        let mut s = String::new();
        let lexs = lexer.lex(source).unwrap();
        print!("lexemes: ");
        for (i, (l, r)) in lexs.iter().zip(values.iter()).enumerate() {
            let l_ty = lexer.get_type(l.item);
            print!("{:?} ({l_ty}) ", l.from_code(source));
            if l_ty != *r {
                s.push_str(&format!("\t{i}. {:?} ({l_ty}) != {r}", l.from_code(source)));
            }
        }
        assert_eq!(
            lexs.len(),
            values.len(),
            "token count mismatch: got {}, expected {}",
            lexs.len(),
            values.len()
        );
        if !s.is_empty() {
            panic!("lexer mismatch:\n{s}");
        }
    }

    pub mod idents {
        use super::{Lexer, expected};

        #[test]
        pub fn aggregate() {
            expected(
                &Lexer::default(),
                "plain snake_case PascalCase UPPER_SNAKE_CASE MiXeD WithNumb3r _123IsValid s O Nice",
                &["Ident"; 10],
            );
        }

        #[test]
        pub fn plain() {
            expected(&Lexer::default(), "plain", &["Ident"]);
        }
    }
}
