use crate::{
    ast::{Ast, Node},
    lexer::{self, TkTy, Token},
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

pub struct Parser {
    pub tokens: Vec<Token>,
    pub idx: usize,
}

impl Parser {
    pub fn cleared(mut self) -> Self {
        self.idx = 0;
        self
    }

    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, idx: 0 }
    }

    pub fn current(&self) -> Result<&Token> {
        self.peek(0)
    }

    pub fn peek(&self, offset: usize) -> Result<&Token> {
        self.tokens
            .get(self.idx + offset)
            .ok_or_else(|| Error::UnexpectedEof {
                at: self.tokens.last().map(|t| t.at).unwrap_or((0..=0).into()),
            })
    }

    pub fn adv(&mut self) -> Option<&Token> {
        self.idx += 1;
        self.tokens.get(self.idx - 1)
    }

    pub fn check(&mut self, f: impl Fn(&Token) -> bool) -> Result<bool> {
        Ok(if f(self.current()?) {
            self.idx += 1;
            true
        } else {
            false
        })
    }

    pub fn syntax(&mut self, tk: TkTy) -> Result<&Token> {
        let peek = self.current()?;
        if peek.item == tk {
            self.idx += 1;
            Ok(self.current()?)
        } else {
            Err(Error::UnexpectedToken {
                exp: tk,
                tk: peek.item.clone(),
                at: peek.at,
            })
        }
    }

    pub fn parse(&mut self) -> Result<Node> {
        self.parse_app()
    }

    pub fn parse_program(&mut self) -> Result<Node> {
        let mut steps = Vec::new();
        let start = self.current()?.at;
        while self.idx < self.tokens.len() {
            steps.push(self.parse_step()?);
        }
        let end = steps.last().map(|n| n.at).unwrap_or(start);
        Ok(Ast::Program(steps).at(lexer::over(start, end)))
    }

    pub fn parse_step(&mut self) -> Result<Node> {
        let ident = self.syntax(TkTy::Ident)?.at;
        let mut params = Vec::new();
        loop {
            match self.syntax(TkTy::Ident) {
                Ok(p) => params.push(p.at),
                Err(Error::UnexpectedToken { .. }) => break,
                Err(e) => return Err(e),
            }
        }
        self.syntax(TkTy::Assign)?;
        let body = self.parse_app()?;
        let span = lexer::over(ident, body.at);
        Ok(Ast::Def {
            ident,
            params,
            body,
        }
        .at(span))
    }

    pub fn parse_abs(&mut self) -> Result<Node> {
        self.syntax(TkTy::Function)?;

        let mut params = Vec::new();

        while self.current()?.item == TkTy::Ident {
            let var_span = self.current()?.at;
            self.idx += 1;
            params.push(var_span);
        }

        if params.is_empty() {
            self.syntax(TkTy::Ident)?;
        }

        self.syntax(TkTy::Abstraction)?;
        let body = self.parse_app()?;

        let mut result = body;
        for param_span in params.into_iter().rev() {
            let at = lexer::over(param_span, result.at);
            result = Ast::Abs(param_span, result).at(at);
        }

        Ok(result)
    }

    pub fn parse_app(&mut self) -> Result<Node> {
        let mut l = self.parse_atom()?;
        while !self.check(|tk| tk.item == TkTy::Sep)? {
            let r = self.parse_atom()?;
            let at = lexer::over(l.at, r.at);
            l = Ast::App(l, r).at(at);
        }
        Ok(l)
    }

    pub fn parse_atom(&mut self) -> Result<Node> {
        if self.current()?.item == TkTy::Function {
            return self.parse_abs();
        }
        if self.check(|t| t.item == TkTy::LParen)? {
            let atom = self.parse_app()?;
            self.syntax(TkTy::RParen)?;
            Ok(atom)
        } else {
            let next_span = self.current()?.at;
            self.syntax(TkTy::Ident)?;
            let node = Ast::Var.at(next_span);
            Ok(node)
        }
    }
}
