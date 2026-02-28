use miette::SourceSpan;

use crate::lexer::Meta;

pub type Node = Box<Meta<Ast>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Abs(SourceSpan, Node),
    App(Node, Node),
    Var,
}

pub fn display_node(n: &Node) {
    fn span_str(span: &SourceSpan) -> String {
        format!("{}..{}", span.offset(), span.offset() + span.len())
    }

    fn indented(n: &Node, mut depth: usize) {
        print!("{}", " ".repeat(depth * 2));
        depth += 1;
        let span = span_str(&n.at);
        match &n.item {
            Ast::Var => println!("ν @ {span}"),
            Ast::Abs(v, inner) => {
                println!("λ {} @ {span} ∈", span_str(v));
                indented(inner, depth);
            }
            Ast::App(l, r) => {
                println!("⋅ @ {span}");
                indented(l, depth);
                indented(r, depth);
            }
        }
    }
    indented(n, 0)
}

impl Ast {
    pub fn at(self, at: SourceSpan) -> Node {
        Meta { item: self, at }.into()
    }
}
