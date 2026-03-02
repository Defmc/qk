use std::{collections::HashMap, fmt};

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::{
    ast::{Ast, Node},
    ir::{self, IrComponent, IrObj, Scope},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TermIdx(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OuterIdx(usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    #[error(transparent)]
    #[diagnostic(transparent)]
    IrCompiler(#[from] crate::ir::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct CodeUnit<'a> {
    pub pool: Vec<Term>,
    pub scope: &'a mut Scope,
    pub src: &'a str,
    pub obj_cache: HashMap<ir::Id, TermIdx>,
    pub layer_stack: Vec<ir::Id>,
}

pub fn print_pool(pool: &[Term]) {
    print!("[ ");
    for (i, t) in pool.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        match t {
            Term::Var(OuterIdx(idx)) => print!("ν{idx}"),
            Term::Abs {
                inner: TermIdx(idx),
            } => print!("λ{idx}"),
            Term::App(TermIdx(l), TermIdx(r)) => print!("{l}⋅{r}"),
        }
    }
    print!(" ]");
}

impl<'a> CodeUnit<'a> {
    pub fn new(scope: &'a mut Scope, src: &'a str) -> Result<Self> {
        scope.check_for_pendings()?;
        let s = Self {
            pool: Vec::new(),
            scope,
            src,
            obj_cache: HashMap::new(),
            layer_stack: Vec::new(),
        };
        Ok(s)
    }

    pub fn compile(&mut self, ir: &IrObj) -> Result<TermIdx> {
        // match &ir.item {
        //     Ast::Var => {
        //         let var_name = ir.from_code(src);
        //         let (id, _) = scopes
        //             .iter()
        //             .rev()
        //             .enumerate()
        //             .find(|(_, s)| **s == var_name)
        //             .ok_or(Error::UndeclaredVariable { at: ir.at })?;
        //         Ok(self.push(Term::Var(OuterIdx(id))))
        //     }
        //     Ast::Abs(v, inner) => {
        //         let var_name = &src[v.offset()..v.offset() + v.len()];
        //         scopes.push(var_name);
        //         let inner = self.compile_node(scopes, inner, src)?;
        //         scopes.pop();
        //         Ok(self.push(Term::Abs { inner }))
        //     }
        //     Ast::App(l, r) => {
        //         let l = self.compile_node(scopes, l, src)?;
        //         let r = self.compile_node(scopes, r, src)?;
        //         Ok(self.push(Term::App(l, r)))
        //     }
        //     _ => todo!(),
        // }
        match &ir.item {
            IrComponent::Pending => {
                unreachable!("`Scope::check_for_pendings` wasn't executed or is bugged")
            }
            IrComponent::Binding => {
                unreachable!("this `TermIdx` shouldn't be the entry point for compilation")
            }
            IrComponent::Abs(id, body) => {
                self.layer_stack.push(*id);
                let inner = self.compile(body)?;
                self.layer_stack.pop();
                Ok(self.push(Term::Abs { inner }))
            }
            IrComponent::App(l, r) => {
                let l = self.compile(l)?;
                let r = self.compile(r)?;
                Ok(self.push(Term::App(l, r)))
            }
            IrComponent::Var(id) => match &self.scope.res_pool[id.0].item {
                IrComponent::Def { .. } => self.compile_resource(*id),
                IrComponent::Binding => {
                    let outer_idx = self
                        .layer_stack
                        .iter()
                        .rev()
                        .enumerate()
                        .find(|(_, sid)| *sid == id)
                        .unwrap()
                        .0;
                    Ok(self.push(Term::Var(OuterIdx(outer_idx))))
                }
                _ => unreachable!(),
            },
            IrComponent::Def { .. } => unreachable!(
                "`IrComponent::Def` should be a guard over a definition, not a IrC output"
            ),
        }
    }

    pub fn compile_resource(&mut self, res_id: ir::Id) -> Result<TermIdx> {
        if let Some(idx) = self.obj_cache.get(&res_id) {
            Ok(*idx)
        } else {
            // TODO remove this clone
            let res = self.scope.res_pool[res_id.0].clone();
            let compiled = self.compile(&res)?;
            self.obj_cache.insert(res_id, compiled);
            Ok(compiled)
        }
    }

    pub fn push(&mut self, t: Term) -> TermIdx {
        self.pool.push(t);
        TermIdx(self.pool.len() - 1)
    }
}
