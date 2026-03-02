use std::collections::HashMap;

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::ast::{Ast, Node};

pub type IrObj = Box<IrComponent>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(usize);

#[derive(Default, Debug)]
pub enum IrComponent {
    Var(Id),
    App(IrObj, IrObj),
    Abs(Id, IrObj),

    /// a definition
    /// i. e, a ident that represents another IrComponent
    /// e. g, I = \x.x, where I is the Def for I and \x.x the IrObj
    Def,

    #[default]
    Pending,

    /// a variable binding
    /// i. e, indicates it's a variable from a lambda expr
    /// e. g, \y.x, where y is a variable binding
    Binding,
}

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("undeclared variable")]
    #[diagnostic(code(ir::undeclared_variable), help("perhaps was a mistyping?"))]
    UndeclaredVariable {
        #[label("this ident is unknown here")]
        at: SourceSpan,
    },

    #[error("forbidden expression placement")]
    #[diagnostic(
        code(ir::forbidden_expr_placement),
        help("if you want to execute this snippet, put inside a `main` entrypoint")
    )]
    ForbiddenExprPlacement {
        #[label("this shouldn't be here")]
        at: SourceSpan,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
/// the first
#[derive(Default)]
pub struct IrCompiler {
    scope: Scope,
}

impl IrCompiler {
    pub fn compile(&mut self, ast: Node, src: &str) -> Result<IrObj> {
        match ast.item {
            Ast::Var => Ok(IrComponent::Var(self.scope.get(ast.from_code(src))).into()),
            Ast::App(l, r) => {
                Ok(IrComponent::App(self.compile(l, src)?, self.compile(r, src)?).into())
            }
            Ast::Abs(v, inner) => self.guard(crate::lexer::from_code(v, src), |s, _id| {
                s.compile(inner, src)
            }),
            Ast::Def {
                ident,
                params,
                body,
            } => {
                let reorganized_abs = params.into_iter().rev().fold(body, |abs, param| {
                    let abs_at = abs.at;
                    crate::lexer::Meta {
                        item: Ast::Abs(param, abs),
                        at: crate::lexer::over(param, abs_at),
                    }
                    .into()
                });
                let inner = self.compile(reorganized_abs, src)?;
                self.scope
                    .push(crate::lexer::from_code(ident, src).into(), inner);
                Ok(IrComponent::Def.into())
            }
            Ast::Program(..) => unimplemented!(),
        }
    }

    pub fn compile_program(&mut self, ast: Node, src: &str) -> Result<()> {
        if let Ast::Program(steps) = ast.item {
            for step in steps {
                match &step.item {
                    Ast::Var | Ast::App(..) | Ast::Abs(..) => {
                        return Err(Error::ForbiddenExprPlacement { at: step.at });
                    }
                    Ast::Def { .. } => {
                        self.compile(step, src)?;
                    }
                    Ast::Program(..) => unreachable!(),
                }
            }
            Ok(())
        } else {
            unimplemented!()
        }
    }

    pub fn guard<T>(&mut self, name: &str, f: impl FnOnce(&mut Self, Id) -> T) -> T {
        let id = self.scope.push_res(IrComponent::Binding.into());
        let old_id = if let Some(old_id) = self.scope.definitions.get_mut(name) {
            let mut new_id = id;
            std::mem::swap(&mut new_id, old_id);
            Some(new_id)
        } else {
            None
        };
        let r = f(self, id);
        if let Some(old_id) = old_id {
            *self.scope.definitions.get_mut(name).unwrap() = old_id;
        } else {
            self.scope.definitions.remove(name);
        }
        r
    }
}

#[derive(Default)]
pub struct Scope {
    res_pool: Vec<IrObj>,
    definitions: HashMap<Box<str>, Id>,
}

impl Scope {
    pub fn push(&mut self, name: Box<str>, res: IrObj) -> Id {
        let id = self.push_res(res);
        self.definitions.insert(name.into(), id);
        id
    }

    pub fn push_res(&mut self, res: IrObj) -> Id {
        let id = Id(self.res_pool.len());
        self.res_pool.push(res);
        id
    }

    /// if `Scope` doesn't have a resource, it doesn't mean the resource doesn't exist
    /// it only means it hasn't been evaluated yet
    pub fn get(&mut self, s: &str) -> Id {
        if let Some(id) = self.definitions.get(s) {
            return *id;
        } else {
            self.push(s.into(), IrComponent::Pending.at((0..=0).into()))
        }
    }

    /// ensures the IR contains all the definitions it needs to compile itself
    pub fn check_for_pendings(&mut self) -> Result<()> {
        for (_k, v) in self.definitions.iter() {
            let comp = &self.res_pool[v.0];
            if comp.item == IrComponent::Pending {
                return Err(Error::UndeclaredVariable { at: comp.at });
            }
        }
        Ok(())
    }
}
