use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::{
    arts::{CompArtifact, OuterIdx, Term, TermIdx},
    ir::{self, IrComponent, IrObj, Scope},
};

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

    pub fn compile(&mut self, ir: &IrObj) -> Result<()> {
        let idx = self.compile_node(ir)?;
        self.art.root = Some(idx);
        Ok(())
    }

    pub fn compile_node(&mut self, ir: &IrObj) -> Result<TermIdx> {
        match &ir.item {
            IrComponent::Pending => {
                unreachable!("`Scope::check_for_pendings` wasn't executed or is bugged")
            }
            IrComponent::Binding => {
                unreachable!("this `TermIdx` shouldn't be the entry point for compilation")
            }
            IrComponent::Abs(id, body) => {
                self.layer_stack.push(*id);
                let inner = self.compile_node(body)?;
                self.layer_stack.pop();
                Ok(self.art.push(Term::Abs { inner }))
            }
            IrComponent::App(l, r) => {
                let l = self.compile_node(l)?;
                let r = self.compile_node(r)?;
                Ok(self.art.push(Term::App(l, r)))
            }
            IrComponent::Var(id) => match &self.scope.res_pool[id.0].item {
                IrComponent::Def { .. } => self.cache_hit_or_compile(*id),
                IrComponent::Binding => {
                    let outer_idx = self
                        .layer_stack
                        .iter()
                        .rev()
                        .enumerate()
                        .find(|(_, sid)| *sid == id)
                        .unwrap()
                        .0;
                    Ok(self.art.push(Term::Var(OuterIdx(outer_idx))))
                }
                _ => unreachable!(),
            },
            IrComponent::Def(obj) => self.compile_node(obj),
        }
    }

    pub fn cache_hit_or_compile(&mut self, res_id: ir::Id) -> Result<TermIdx> {
        if let Some(idx) = self.art.obj_cache.get(&res_id) {
            Ok(*idx)
        } else {
            let res = &self.scope.res_pool[res_id.0];
            let compiled = self.compile_node(res)?;
            self.art.obj_cache.insert(res_id, compiled);
            Ok(compiled)
        }
    }
}
