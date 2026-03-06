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
        s.push_str("[ ");
        for (i, t) in self.arena.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            let _ = match t {
                Term::Var(OuterIdx(idx)) => write!(s, "[{i}]=ν{idx}"),
                Term::Abs {
                    inner: TermIdx(idx),
                } => write!(s, "[{i}]=λ{idx}"),
                Term::App(TermIdx(l), TermIdx(r)) => write!(s, "[{i}]={l}⋅{r}"),
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
            } else {
                s.push(' ');
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
            "arena: {} | cache: {} | {}",
            self.arena_to_string(),
            self.obj_cache_to_string(aliases),
            match self.root {
                Some(r) => format!("root: {}", r.0),
                None => "lib".into(),
            }
        )
    }
    pub fn push(&mut self, t: Term) -> TermIdx {
        let idx = self.arena.len();
        self.arena.push(t);
        TermIdx(idx)
    }

    pub fn get(&self, i: TermIdx) -> Term {
        self.arena[i.0].clone()
    }

    pub fn pretty_print(&self, idx: TermIdx, aliases: &HashMap<ir::Id, Box<str>>) {
        let mut layers = Vec::new();
        let mut inverse_cache = self.obj_cache.iter().map(|(ir, ti)| (*ti, *ir)).collect();
        self.pretty_print_inner(idx, &mut inverse_cache, &mut layers, aliases);
        println!();
    }

    fn pretty_print_inner(
        &self,
        idx: TermIdx,
        inverse_cache: &HashMap<TermIdx, ir::Id>,
        abs_layers: &mut Vec<usize>,
        aliases: &HashMap<ir::Id, Box<str>>,
    ) {
        if let Some(alias) = inverse_cache.get(&idx).and_then(|i| aliases.get(i)) {
            print!("{alias}");
            return;
        }
        match self.get(idx) {
            Term::Var(v) => print!(
                "{}",
                abs_layers
                    .len()
                    .checked_sub(v.0 + 1)
                    .and_then(|n| abs_layers.get(n))
                    .map_or_else(|| "?".to_string(), |&v| ir::Scope::id_to_str(&ir::Id(v)))
            ),
            Term::App(l, r) => {
                self.pretty_print_inner(l, inverse_cache, abs_layers, aliases);
                print!(" ");
                if let Term::App(..) = self.get(r) {
                    print!("(");
                    self.pretty_print_inner(r, inverse_cache, abs_layers, aliases);
                    print!(")");
                } else {
                    self.pretty_print_inner(r, inverse_cache, abs_layers, aliases);
                }
            }
            Term::Abs { inner } => {
                abs_layers.push(idx.0);
                print!(
                    "λ{}.",
                    ir::Scope::id_to_str(&ir::Id(*abs_layers.last().unwrap()))
                );
                self.pretty_print_inner(inner, inverse_cache, abs_layers, aliases);
                abs_layers.pop();
            }
        }
    }
}
