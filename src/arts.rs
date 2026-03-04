use crate::ir;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TermIdx(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OuterIdx(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
    Var(OuterIdx),
    Abs { inner: TermIdx },
    App(TermIdx, TermIdx),
}

/// the goat.
/// lambda calculus is (beautifully) referentially transparent
/// if a b -> c, and a = x, b = y, so x y -> c
/// every calculation will always return the same value,
/// regardless of the moment it's executed
/// `Compiler Artifact` handles the job of ensuring everything
/// done can be reutilized
#[derive(Default, Debug)]
pub struct CompArtifact {
    arena: Vec<Term>,
    pub obj_cache: HashMap<ir::Id, TermIdx>,
    pub root: Option<TermIdx>,
}

impl CompArtifact {
    pub fn arena(&self) -> &[Term] {
        &self.arena
    }

    pub fn arena_to_string(&self) -> String {
        let mut s = String::new();
        s.push_str("[");
        for (i, t) in self.arena.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            let _ = match t {
                Term::Var(OuterIdx(idx)) => write!(s, " [{i}]=ν{idx}"),
                Term::Abs {
                    inner: TermIdx(idx),
                } => write!(s, " [{i}]=λ{idx}"),
                Term::App(TermIdx(l), TermIdx(r)) => write!(s, " [{i}]={l}⋅{r}"),
            };
        }
        s.push_str(" ]");
        s
    }

    pub fn obj_cache_to_string(&self, aliases: &HashMap<ir::Id, Box<str>>) -> String {
        let mut s = String::new();
        let use_alias = !aliases.is_empty();
        s.push_str("{");
        for (i, (id, term_idx)) in self.obj_cache.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            if let Some(Some(name)) = use_alias.then(|| aliases.get(&id)) {
                s.push_str(name)
            } else {
                let _ = write!(s, "{}", id.0);
            }
            let _ = write!(s, " => {}", term_idx.0);
        }
        s.push_str(" }");
        s
    }

    pub fn to_string(&self, aliases: &HashMap<ir::Id, Box<str>>) -> String {
        format!(
            "arena: {} | cache: {}",
            self.arena_to_string(),
            self.obj_cache_to_string(aliases)
        )
    }
    pub fn push(&mut self, t: Term) -> TermIdx {
        let idx = self.arena.len();
        self.arena.push(t);
        TermIdx(idx)
    }
}
