use std::fmt::Write;
use std::{collections::HashMap, fmt};

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::ir::{self, IrComponent, IrObj, Scope};

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

#[derive(Default, Debug)]
pub struct CompArtifact {
    arena: Vec<Term>,
    pub obj_cache: HashMap<ir::Id, TermIdx>,
}

impl CompArtifact {
    pub fn arena(&self) -> &[Term] {
        &self.arena
    }

    pub fn arena_to_string(&self) -> String {
        let mut s = String::new();
        s.push_str("[ ");
        for (i, t) in self.arena.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            let _ = match t {
                Term::Var(OuterIdx(idx)) => write!(s, "[{i}]=ν{idx}"),
                Term::Abs {
                    inner: TermIdx(idx),
                } => write!(s, " [{i}]=λ{idx}"),
                Term::App(TermIdx(l), TermIdx(r)) => write!(s, "[{i}]={l}⋅{r}"),
            };
        }
        s.push_str("]");
        s
    }

    pub fn obj_cache_to_string(&self, aliases: &HashMap<ir::Id, Box<str>>) -> String {
        let mut s = String::new();
        let use_alias = !aliases.is_empty();
        s.push_str("{ ");
        for (i, (id, term_idx)) in self.obj_cache.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push(' ');
            if let Some(Some(name)) = use_alias.then(|| aliases.get(&id)) {
                s.push_str(name)
            } else {
                let _ = write!(s, "{}", id.0);
            }
            let _ = write!(s, " => {}", term_idx.0);
        }
        s.push_str("}");
        s
    }

    pub fn to_string(&self, aliases: &HashMap<ir::Id, Box<str>>) -> String {
        format!(
            "arena: {} | cache: {}",
            self.arena_to_string(),
            self.obj_cache_to_string(aliases)
        )
    }
}

#[derive(Debug)]
pub struct CodeUnit<'a> {
    pub scope: &'a Scope,
    pub src: &'a str,
    pub art: CompArtifact,
    pub layer_stack: Vec<ir::Id>,
}

impl<'a> CodeUnit<'a> {
    pub fn new(scope: &'a mut Scope, src: &'a str) -> Result<Self> {
        Self::with_artifacts(scope, src, CompArtifact::default())
    }

    pub fn with_artifacts(scope: &'a mut Scope, src: &'a str, art: CompArtifact) -> Result<Self> {
        scope.check_for_pendings()?;
        let s = Self {
            art,
            scope,
            src,
            layer_stack: Vec::new(),
        };
        Ok(s)
    }

    pub fn compile(&mut self, ir: &IrObj) -> Result<TermIdx> {
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
            IrComponent::Def(obj) => self.compile(obj),
        }
    }

    pub fn compile_resource(&mut self, res_id: ir::Id) -> Result<TermIdx> {
        if let Some(idx) = self.art.obj_cache.get(&res_id) {
            Ok(*idx)
        } else {
            let res = &self.scope.res_pool[res_id.0];
            let compiled = self.compile(res)?;
            self.art.obj_cache.insert(res_id, compiled);
            Ok(compiled)
        }
    }

    pub fn push(&mut self, t: Term) -> TermIdx {
        self.art.arena.push(t);
        TermIdx(self.art.arena.len() - 1)
    }
}
