use std::collections::HashMap;

use crate::arts::{CompArtifact, OuterIdx, Term, TermIdx};

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
        // if let Some(idx) = self.reductions.get(&(inner, with)) {
        //     *idx
        // } else {
        let t = self.substitute_inner(inner, with, 0).unwrap_or(inner);
        // self.reductions.insert((inner, with), t);
        self.shift(t, -1)
        // }
    }

    fn substitute_inner(&mut self, abs: TermIdx, with: TermIdx, layer: usize) -> Option<TermIdx> {
        match self.art.get(abs) {
            Term::Var(o) if o.0 == layer => Some(with),
            Term::Var(..) => None,
            Term::App(l, r) => {
                let new_l = self.substitute_inner(l, with, layer).unwrap_or(l);
                let new_r = self.substitute_inner(r, with, layer).unwrap_or(r);
                if new_l != l || new_r != r {
                    Some(self.art.push(Term::App(new_l, new_r)))
                } else {
                    None
                }
            }
            Term::Abs { inner } => {
                let with_shifted = self.shift(with, 1);
                self.substitute_inner(inner, with_shifted, layer + 1)
                    .and_then(|inner| Some(self.art.push(Term::Abs { inner })))
            }
        }
    }

    pub fn shift(&mut self, term: TermIdx, layers: isize) -> TermIdx {
        self.shift_inner(term, 0, layers).unwrap_or(term)
    }

    fn shift_inner(
        &mut self,
        term: TermIdx,
        current_layer: usize,
        layers: isize,
    ) -> Option<TermIdx> {
        match self.art.get(term) {
            Term::Var(o) if o.0 >= current_layer => Some(
                self.art
                    .push(Term::Var(OuterIdx(o.0.strict_add_signed(layers)))),
            ),
            Term::Var(..) => None,
            Term::Abs { inner } => self
                .shift_inner(inner, current_layer + 1, layers)
                .and_then(|inner| Some(self.art.push(Term::Abs { inner }))),
            Term::App(l, r) => {
                let new_l = self.shift_inner(l, current_layer, layers).unwrap_or(l);
                let new_r = self.shift_inner(r, current_layer, layers).unwrap_or(r);
                if new_l != l || new_r != r {
                    Some(self.art.push(Term::App(new_l, new_r)))
                } else {
                    None
                }
            }
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
            Term::Abs { inner } => match Self::step(c, inner) {
                Op::Reduced(new_inner) => Op::Reduced(c.art.push(Term::Abs { inner: new_inner })),
                op => op,
            },
            Term::App(l, r) => {
                if let Term::Abs { inner } = c.art.get(l) {
                    return Op::Reduced(c.substitute(inner, r));
                }

                match Self::step(c, l) {
                    Op::Reduced(redex_l) => Op::Reduced(c.art.push(Term::App(redex_l, r))),
                    Op::Normal => Self::step(c, r),
                    op => op,
                }
            }
        }
    }
}
