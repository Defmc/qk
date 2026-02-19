use std::collections::HashMap;

use crate::{
    Body, Term,
    ast::{Ast, Node},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VarId(usize);

#[derive(Default)]
pub struct Compiler {
    definitions: HashMap<String, VarId>,
    var_counter: usize,
}

pub struct ScopeGuard<'a> {
    shadowed: Option<VarId>,
    compiler: &'a mut Compiler,
    var_name: &'a str,
    pub var_id: VarId,
}

impl<'a> ScopeGuard<'a> {
    pub fn new(compiler: &'a mut Compiler, var_name: &'a str) -> Self {
        let var_id = compiler.get_new_var();
        let shadowed = if let Some(shadow) = compiler.definitions.get_mut(var_name) {
            Some(std::mem::replace(shadow, var_id))
        } else {
            compiler.definitions.insert(var_name.to_string(), var_id);
            None
        };
        Self {
            shadowed,
            compiler,
            var_id,
            var_name,
        }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        if let Some(shadow) = self.shadowed {
            *self.compiler.definitions.get_mut(self.var_name).unwrap() = shadow;
        } else {
            self.compiler.definitions.remove(self.var_name);
        }
    }
}

impl Compiler {
    pub fn compile(&mut self, ast: Node, src: &str) -> Term {
        match ast.item {
            Ast::Var => {
                let v = self
                    .definitions
                    .get(ast.from_code(src))
                    .expect("variable not defined");
                Body::Var(v.0).into()
            }
            Ast::Abs(v, t) => {
                let var_text = &src[v.offset()..v.offset() + v.len()];
                let new_v = ScopeGuard::new(self, var_text);
                Body::Abs(new_v.var_id.0, self.compile(t, src)).into()
            }
            Ast::App(l, r) => Body::App(self.compile(l, src), self.compile(r, src)).into(),
        }
    }

    pub fn get_new_var(&mut self) -> VarId {
        self.var_counter += 1;
        VarId(self.var_counter - 1)
    }
}
