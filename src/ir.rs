use std::{
    collections::{HashMap, hash_map::Entry},
    sync::LazyLock,
};

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::{
    ast::{Ast, Node},
    lexer::{Meta, Trace},
};

pub type IrObj = Box<Meta<IrComponent>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum IrComponent {
    Var(Id),
    App(IrObj, IrObj),
    Abs(Id, IrObj),

    /// a definition
    /// i. e, a ident that represents another IrComponent
    /// e. g, I = \x.x, where I is the Def for I and \x.x the IrObj
    Def(IrObj),

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

    #[error("duplicated definition of {name:?}")]
    #[diagnostic(
        code(ir::duplicated_definition),
        help("shadowing is only allow in function scopes")
    )]
    DuplicatedDefinition {
        name: Box<str>,

        #[label("{name:?} is first defined here")]
        first: SourceSpan,

        #[label("afterwards, it's again defined here")]
        second: SourceSpan,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
/// the first
#[derive(Default, Debug)]
pub struct IrCompiler {
    pub scope: Scope,
}

impl IrCompiler {
    pub fn compile(&mut self, ast: Node, src: &str) -> Result<IrObj> {
        match ast.item {
            Ast::Var => {
                Ok(IrComponent::Var(self.scope.get_or_reserve(ast.from_code(src))?).at(ast.at))
            }
            Ast::App(l, r) => {
                Ok(IrComponent::App(self.compile(l, src)?, self.compile(r, src)?).at(ast.at))
            }
            Ast::Abs(v, inner) => self.guard(crate::lexer::from_code(v, src), ast.at, |s, id| {
                Ok(IrComponent::Abs(id, s.compile(inner, src)?).at(ast.at))
            }),
            Ast::Def { .. } | Ast::Program(..) => unimplemented!(),
        }
    }

    pub fn compile_program(&mut self, ast: Node, src: &str) -> Result<()> {
        if let Ast::Program(steps) = ast.item {
            for step in steps {
                match step.item {
                    Ast::Var | Ast::App(..) | Ast::Abs(..) => {
                        return Err(Error::ForbiddenExprPlacement { at: step.at });
                    }
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
                        let obj = IrComponent::Def(inner).at(step.at);
                        self.scope
                            .push(crate::lexer::from_code(ident, src).into(), obj)?;
                    }
                    Ast::Program(..) => unreachable!(),
                }
            }
            Ok(())
        } else {
            unimplemented!()
        }
    }

    pub fn guard<T>(
        &mut self,
        name: &str,
        binding_span: SourceSpan,
        f: impl FnOnce(&mut Self, Id) -> T,
    ) -> T {
        let id = self.scope.push_res(IrComponent::Binding.at(binding_span));
        let old_id = if let Some(old_id) = self.scope.definitions.get_mut(name) {
            let mut new_id = id;
            std::mem::swap(&mut new_id, old_id);
            Some(new_id)
        } else {
            self.scope.definitions.insert(name.into(), id);
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

#[derive(Default, Debug)]
pub struct Scope {
    pub res_pool: Vec<IrObj>,
    pub definitions: HashMap<Box<str>, Id>,
}

impl Scope {
    pub fn push(&mut self, name: Box<str>, res: IrObj) -> Result<Id> {
        let id = self.push_res(res);
        match self.definitions.entry(name) {
            Entry::Vacant(e) => {
                e.insert(id);
            }
            Entry::Occupied(mut e) => {
                let old = &self.res_pool[e.get().0];
                if !matches!(old.item, IrComponent::Pending) {
                    let e = Error::DuplicatedDefinition {
                        name: e.key().clone(),
                        first: old.at,
                        second: self.res_pool[id.0].at,
                    };
                    return Err(e);
                } else {
                    e.insert(id);
                }
            }
        }
        Ok(id)
    }

    pub fn push_res(&mut self, res: IrObj) -> Id {
        let id = Id(self.res_pool.len());
        self.res_pool.push(res);
        id
    }

    /// if `Scope` doesn't have a resource, it doesn't mean the resource doesn't exist
    /// it only means it hasn't been evaluated yet
    pub fn get_or_reserve(&mut self, s: &str) -> Result<Id> {
        if let Some(id) = self.definitions.get(s) {
            Ok(*id)
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

    pub fn pretty_print(&self, ir: &IrObj) {
        let mut binding_stack = Vec::new();
        let aliases = self.get_aliases();
        self.buff_pretty_print(&aliases, &mut binding_stack, ir);
    }

    pub fn get_aliases(&self) -> HashMap<Id, Box<str>> {
        self.definitions
            .iter()
            .map(|(l, r)| (*r, l.clone()))
            .collect()
    }

    pub fn buff_pretty_print(
        &self,
        aliases: &HashMap<Id, Box<str>>,
        binding_stack: &mut Vec<Id>,
        ir: &IrObj,
    ) {
        match &ir.item {
            IrComponent::Pending => print!("..."),
            IrComponent::Binding => {
                unreachable!()
            }
            IrComponent::Def(def) => self.buff_pretty_print(aliases, binding_stack, def),
            IrComponent::Var(id) => {
                if let Some(alias) = aliases.get(id) {
                    print!("{alias}")
                } else if let Some(v) = binding_stack.iter().find(|i| id == *i) {
                    print!("{}", Self::id_to_str(v));
                } else {
                    self.buff_pretty_print(aliases, binding_stack, &self.res_pool[id.0])
                }
            }
            IrComponent::App(l, r) => {
                self.buff_pretty_print(aliases, binding_stack, l);
                self.buff_pretty_print(aliases, binding_stack, r);
            }
            IrComponent::Abs(v, inner) => {
                binding_stack.push(*v);
                let id_str = Self::id_to_str(v);
                print!("λ{id_str}.");
                self.buff_pretty_print(aliases, binding_stack, inner);
                binding_stack.pop();
            }
        }
    }

    pub fn id_to_str(id: &Id) -> String {
        const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz";
        static CHARS: LazyLock<Vec<char>> = LazyLock::new(|| ALPHABET.chars().collect());
        let base = CHARS.len();
        let mut n = id.0 + 1;
        let mut s = String::new();
        while n > 0 {
            n -= 1;
            s.push(CHARS[n % base]);
            n /= CHARS.len()
        }
        s
    }
}
