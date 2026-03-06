use crate::{
    ast::{Ast, Node},
    lexer::{self, TkTy, Token, Trace},
};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("unexpected end of file")]
    #[diagnostic(
        code(parser::unexpected_eof),
        help("maybe you've left some parenthesis opened")
    )]
    UnexpectedEof {
        #[label("bit of a sudden, isn't it?")]
        at: SourceSpan,
    },

    #[error("unexpected token {tk:?}")]
    #[diagnostic(
        code(parser::unexpected_token),
        help("so far, we were expecting a {exp:?}")
    )]
    UnexpectedToken {
        exp: TkTy,
        tk: TkTy,
        #[label("here")]
        at: SourceSpan,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
pub type ParseResult<'a, T> = Result<(T, &'a [Token])>;

pub struct Parser<T> {
    parse: Box<dyn for<'a> Fn(&'a [Token]) -> ParseResult<'a, T>>,
}

impl<T: 'static> Parser<T> {
    pub fn new(f: impl Fn(&[Token]) -> ParseResult<T> + 'static) -> Self {
        Self { parse: Box::new(f) }
    }

    pub fn run<'a>(&self, input: &'a [Token]) -> ParseResult<'a, T> {
        (self.parse)(input)
    }

    pub fn map<U: 'static>(self, f: impl Fn(T) -> U + 'static) -> Parser<U> {
        Parser::new(move |input| self.run(input).map(|(v, r)| (f(v), r)))
    }

    pub fn or(self, alternative: Parser<T>) -> Parser<T> {
        Parser::new(move |input| self.run(input).or_else(|_| alternative.run(input)))
    }

    pub fn and_then<U: 'static>(self, f: impl Fn(T) -> Parser<U> + 'static) -> Parser<U> {
        Parser::new(move |input| {
            let (val, rest) = self.run(input)?;
            f(val).run(rest)
        })
    }

    pub fn skip_left<U: 'static>(self, right: Parser<U>) -> Parser<U> {
        Parser::new(move |input| {
            let (_, res) = self.run(input)?;
            right.run(res)
        })
    }

    pub fn skip_right<U: 'static>(self, right: Parser<U>) -> Self {
        Parser::new(move |input| {
            let (val, res) = self.run(input)?;
            let (_, res) = right.run(res)?;
            Ok((val, res))
        })
    }

    pub fn paired<U: 'static>(self, right: Parser<U>) -> Parser<(T, U)> {
        Parser::new(move |input| {
            let (val_l, res) = self.run(input)?;
            let (val_r, res) = right.run(res)?;
            Ok(((val_l, val_r), res))
        })
    }

    pub fn many(self, min: usize) -> Parser<Vec<T>> {
        Parser::new(move |mut input| {
            let mut vs = Vec::new();
            while let Ok((v, res)) = self.run(input) {
                vs.push(v);
                input = res;
            }
            if vs.len() < min {
                Err(Error::UnexpectedEof { at: (0..=0).into() })
            } else {
                Ok((vs, input))
            }
        })
    }

    pub fn lazy(f: impl Fn() -> Self + 'static) -> Self {
        Parser::new(move |input| f().run(input))
    }

    pub fn one_of(branches: Vec<Self>) -> Self {
        Parser::new(move |input| {
            let mut branches = branches.iter();
            let mut last = branches.next().expect("`one_of` is empty");
            let mut v = last.run(input);
            loop {
                if v.is_ok() {
                    break;
                }
                if let Some(next_p) = branches.next() {
                    last = next_p;
                    v = last.run(input);
                } else {
                    break;
                }
            }
            v
        })
    }
}
pub fn syntax<'a>(tk: TkTy) -> Parser<Token> {
    Parser::new(move |input| {
        let first = input
            .first()
            .ok_or_else(|| Error::UnexpectedEof { at: (0..=0).into() })?
            .clone();
        if first.item == tk {
            Ok((first, &input[1..]))
        } else {
            Err(Error::UnexpectedToken {
                exp: tk.clone(),
                at: first.at.clone(),
                tk: first.item.clone(),
            })
        }
    })
}

pub fn program() -> Parser<Node> {
    Parser::many(decl().skip_right(Parser::many(syntax(TkTy::Sep), 1)), 0).map(|decls| {
        let start = decls.first().map(|d| d.at.offset() as usize).unwrap_or(0);
        let end = decls.last().map(|d| d.at.offset() as usize).unwrap_or(0);
        Ast::Program(decls).at((start..=end).into())
    })
}

pub fn decl() -> Parser<Node> {
    syntax(TkTy::Ident)
        .paired(Parser::many(syntax(TkTy::Ident), 1))
        .skip_right(syntax(TkTy::Assign))
        .paired(expr())
        .map(|((d, vars), expr)| {
            let span = lexer::over(d.at, expr.at);
            Ast::Def {
                ident: d.at,
                params: vars.into_iter().map(|tk| tk.at).collect(),
                body: expr,
            }
            .at(span)
        })
}

pub fn expr() -> Parser<Node> {
    app()
}

pub fn app() -> Parser<Node> {
    Parser::many(Parser::lazy(atom), 1).map(|atoms| {
        let mut atoms = atoms.into_iter();
        let first = atoms.next().unwrap();
        atoms.fold(first, |l, r| {
            let span = lexer::over(l.at, r.at);
            Ast::App(l, r).at(span)
        })
    })
}

pub fn abs() -> Parser<Node> {
    syntax(TkTy::Function)
        .skip_left(syntax(TkTy::Ident))
        .paired(
            syntax(TkTy::Abstraction)
                .skip_left(Parser::lazy(atom))
                .or(Parser::lazy(abs)),
        )
        .map(move |(var, inner)| {
            let inner_at = inner.at;
            Ast::Abs(var.at, inner).at(lexer::over(var.at, inner_at))
        })
}

pub fn parens() -> Parser<Node> {
    syntax(TkTy::LParen)
        .skip_left(Parser::lazy(atom))
        .skip_right(syntax(TkTy::RParen))
}

pub fn var() -> Parser<Node> {
    syntax(TkTy::Ident).map(move |v| Ast::Var.at(v.at))
}

pub fn atom() -> Parser<Node> {
    Parser::one_of(vec![
        Parser::lazy(parens),
        Parser::lazy(abs),
        Parser::lazy(var),
    ])
}
