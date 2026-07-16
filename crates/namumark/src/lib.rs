//! 나무마크(namumark) 파서.
//!
//! 나무마크 원문을 [`Document`] AST로 변환한다. 렌더링은 별도 크레이트가 담당한다.

mod ast;
mod block;
mod inline;
mod table;

pub use ast::*;

pub fn parse(source: &str) -> Document {
    block::parse_document(source)
}
