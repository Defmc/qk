use miette::SourceSpan;

use crate::lexer::Meta;

pub type Node = Box<Meta<Ast>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Abs(SourceSpan, Node),
    App(Node, Node),
    Var,
}

impl Ast {
    pub fn at(self, at: SourceSpan) -> Node {
        Meta { item: self, at }.into()
    }
}
