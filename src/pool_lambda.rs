use std::fmt;

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::ast::{Ast, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TermIdx(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OuterIdx(usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    Var(OuterIdx),
    Abs { inner: TermIdx },
    App(TermIdx, TermIdx),
}

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("undeclared variable")]
    #[diagnostic(
        code(compiler::pool::undeclared_variable),
        help("perhaps was a mistyping?")
    )]
    UndeclaredVariable {
        #[label("this ident is unknown here")]
        at: SourceSpan,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct Pool {
    pub pool: Vec<Term>,
}

impl fmt::Display for Pool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[ ")?;
        for (i, t) in self.pool.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            match t {
                Term::Var(OuterIdx(idx)) => write!(f, "ν{idx}")?,
                Term::Abs {
                    inner: TermIdx(idx),
                } => write!(f, "λ{idx}")?,
                Term::App(TermIdx(l), TermIdx(r)) => write!(f, "{l}⋅{r}")?,
            }
        }
        f.write_str(" ]")?;
        Ok(())
    }
}

impl Pool {
    pub fn compile(ast: &Node, src: &str) -> Result<Self> {
        let mut s = Self::default();
        let mut scopes = Vec::new();
        s.compile_node(&mut scopes, ast, src)?;
        Ok(s)
    }

    fn compile_node<'a>(
        &mut self,
        scopes: &mut Vec<&'a str>,
        ast: &Node,
        src: &'a str,
    ) -> Result<TermIdx> {
        match &ast.item {
            Ast::Var => {
                let var_name = ast.from_code(src);
                let (id, _) = scopes
                    .iter()
                    .rev()
                    .enumerate()
                    .find(|(_, s)| **s == var_name)
                    .ok_or(Error::UndeclaredVariable { at: ast.at })?;
                Ok(self.push(Term::Var(OuterIdx(id))))
            }
            Ast::Abs(v, inner) => {
                let var_name = &src[v.offset()..v.offset() + v.len()];
                scopes.push(var_name);
                let inner = self.compile_node(scopes, inner, src)?;
                scopes.pop();
                Ok(self.push(Term::Abs { inner }))
            }
            Ast::App(l, r) => {
                let l = self.compile_node(scopes, l, src)?;
                let r = self.compile_node(scopes, r, src)?;
                Ok(self.push(Term::App(l, r)))
            }
        }
    }

    pub fn push(&mut self, t: Term) -> TermIdx {
        self.pool.push(t);
        TermIdx(self.pool.len() - 1)
    }
}
