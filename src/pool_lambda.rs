use std::{collections::HashMap, fmt};

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::ast::{Ast, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TermIdx(usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    Var(TermIdx),
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
        let v: Vec<_> = self
            .pool
            .iter()
            .map(|t| match t {
                Term::Var(id) => format!("ν {id:?}"),
                Term::App(l, r) => format!("{l:?} ⋅ {r:?}"),
                Term::Abs { inner } => format!("λ {inner:?}"),
            })
            .collect();
        f.write_str(&v.join(", "))?;
        f.write_str("]")
    }
}

impl Pool {
    pub fn compile(ast: Node, src: &str) -> Result<Self> {
        let mut s = Self::default();
        let mut scopes = HashMap::default();
        s.compile_node(&mut scopes, &ast, src)?;
        Ok(s)
    }

    fn compile_node<'a>(
        &mut self,
        scopes: &mut HashMap<&'a str, TermIdx>,
        ast: &Node,
        src: &'a str,
    ) -> Result<TermIdx> {
        match &ast.item {
            Ast::Var => {
                let var_name = ast.from_code(src);
                if let Some(id) = scopes.get(var_name) {
                    self.pool.push(Term::Var(*id));
                } else {
                    Err(Error::UndeclaredVariable { at: ast.at })?
                }
            }
            Ast::Abs(v, inner) => {
                let term_idx = self.pool.len();
                self.pool.push(Term::Abs { inner: TermIdx(0) });
                let var_name = &src[v.offset()..v.offset() + v.len()];
                let old_value = scopes.insert(var_name, TermIdx(term_idx));
                let body = self.compile_node(scopes, inner, src)?;
                match old_value {
                    Some(t) => scopes.insert(var_name, t),
                    None => scopes.remove(var_name),
                };
                if let Term::Abs { ref mut inner, .. } = self.pool[term_idx] {
                    *inner = body;
                }
            }
            Ast::App(l, r) => {
                let l = self.compile_node(scopes, l, src)?;
                let r = self.compile_node(scopes, r, src)?;
                self.pool.push(Term::App(l, r));
            }
        }
        Ok(TermIdx(self.pool.len() - 1))
    }
}
