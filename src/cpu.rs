use std::collections::HashMap;

use crate::arts::{CompArtifact, TermIdx};

pub struct EffectReq;

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
}

impl Cpu {
    pub fn new(art: CompArtifact) -> Self {
        Self {
            art,
            reductions: HashMap::new(),
        }
    }
}
