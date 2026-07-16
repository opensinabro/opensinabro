//! 무손실(red-green) 구문 트리.
//!
//! 파이프라인: 원문 → 파서(이벤트 버퍼, 백트래킹 가능) → sink(단방향 재생) → green tree.

mod event;
mod grammar;
mod kind;
mod parser;
mod sink;

pub use kind::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
pub use rowan::NodeOrToken;

/// 원문을 완전무손실 구문 트리로 파싱한다. `tree.text() == source`가 항상 성립한다.
#[derive(Debug, Clone)]
pub struct SyntaxTree {
    root: rowan::GreenNode,
}

impl SyntaxTree {
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }

    pub fn text(&self) -> String {
        self.root().text().to_string()
    }
}

pub fn parse(source: &str) -> SyntaxTree {
    let mut parser = parser::Parser::new(source);
    grammar::document(&mut parser);
    let events = parser.finish();
    SyntaxTree {
        root: sink::build_tree(source, &events),
    }
}
