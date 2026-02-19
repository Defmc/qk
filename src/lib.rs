use std::fmt::Display;

pub mod ast;
pub mod compiler;
pub mod lexer;
pub mod parser;

pub type Term = Box<Body>;

pub enum Body {
    Var(usize),
    App(Term, Term),
    Abs(usize, Term),
}

impl Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(v) => write!(f, "{v}"),
            Self::App(l, r) => {
                if matches!(**l, Self::Abs(..)) {
                    write!(f, "({l}) ")?
                } else {
                    write!(f, "{l} ")?
                }
                if matches!(**r, Self::App(..)) {
                    write!(f, "({r})")
                } else {
                    write!(f, "{r}")
                }
            }
            Self::Abs(v, b) => write!(f, "λ{v}.{b}"),
        }
    }
}
