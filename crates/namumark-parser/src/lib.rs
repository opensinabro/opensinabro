//! 나무마크 파서: 무손실 구문 트리를 의미 모델로 lowering한다.
//!
//! 트리는 구조만 담고 있으므로 leaf 의미(색상 값, 앵커, 셀 옵션 등)는
//! 토큰 텍스트에 표기 판정(namumark-text)을 적용해 계산한다.

mod lower;
mod semantics;

pub use namumark_ast::Document;

/// 나무마크 원문을 의미 모델([`Document`])로 파싱한다.
///
/// 원문 위치 정보가 필요하면 [`namumark_syntax::parse`]로 무손실 구문 트리를
/// 직접 사용하면 된다.
pub fn parse(source: &str) -> Document {
    let tree = namumark_syntax::parse(source);
    lower::lower_document(&tree.root())
}
