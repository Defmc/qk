#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AstNode {
    Token,
}

pub type Ast = AstNode;
