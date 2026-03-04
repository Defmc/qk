use std::collections::HashMap;

use crate::arts::{CompArtifact, Term, TermIdx};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectReq;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    Normal,
    Reduced(TermIdx),
    Effect(EffectReq),
}

/// the actual machine reductor
/// notice the lack of error handling
/// since everything becomes a lambda expression
/// there's no way to fail.
/// it either works, or we have a bug
/// in the case of the latter, we better have guns as well.
pub struct Cpu {
    pub art: CompArtifact,

    /// for a b -> c, maps (a, b) to c
    pub reductions: HashMap<(TermIdx, TermIdx), TermIdx>,
    // how many λ are we into. Should be always zero outside reduction methods
    // pub abs_layer: usize,
}

pub trait Reductor {
    fn step(c: &mut Cpu, idx: TermIdx) -> Op;

    fn reduce(s: &mut Cpu, root: TermIdx) -> TermIdx {
        let mut idx = root;
        while let Op::Reduced(i) = Self::step(s, idx) {
            idx = i;
        }
        idx
    }
}

impl Cpu {
    pub fn new(art: CompArtifact) -> Self {
        Self {
            art,
            reductions: HashMap::new(),
        }
    }

    /// replaces every ocurrence of its index with the idx requested
    /// for \x.x[b], returns b
    pub fn substitute(&mut self, inner: TermIdx, with: TermIdx) -> TermIdx {
        if let Some(idx) = self.reductions.get(&(inner, with)) {
            *idx
        } else {
            let t = self.substitute_inner(inner, with, 0).unwrap_or(inner);
            self.reductions.insert((inner, with), t);
            t
        }
    }

    fn substitute_inner(&mut self, abs: TermIdx, with: TermIdx, layer: usize) -> Option<TermIdx> {
        match self.art.get(abs) {
            Term::Var(o) if o.0 == layer => Some(with),
            Term::Var(..) => None,
            Term::App(l, r) => {
                if self.substitute_inner(l, with, layer).is_some()
                    || self.substitute_inner(r, with, layer).is_some()
                {
                    let new = self.art.push(Term::App(l, r));
                    Some(new)
                } else {
                    None
                }
            }
            Term::Abs { inner } => self
                .substitute_inner(inner, with, layer + 1)
                .and_then(|inner| Some(self.art.push(Term::Abs { inner }))),
        }
    }
}

pub struct Normal;

impl Reductor for Normal {
    fn step(c: &mut Cpu, idx: TermIdx) -> Op {
        match c.art.arena()[idx.0] {
            Term::Var(..) => {
                // `Op::Normal` means that f(a) -> a, so we can say that a variable is normal
                Op::Normal
            }
            Term::Abs { inner } => Self::step(c, inner),
            Term::App(l, r) => {
                let l_step = Self::step(c, l);
                if l_step == Op::Normal
                    && let Term::Abs { inner } = c.art.get(idx)
                {
                    Op::Reduced(c.substitute(inner, r))
                } else {
                    Self::step(c, r)
                }
            }
        }
    }
}
