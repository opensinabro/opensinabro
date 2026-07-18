//! 나무마크 파서 facade.
//!
//! 무손실 구문 트리([`namumark_syntax`])를 파싱해 의미 모델 뷰([`namumark_ast`])로
//! 노출한다. 의미값은 뷰 접근자가 토큰에서 계산하므로 이 계층은 얇은 진입점일 뿐이다.

pub use namumark_ast::Document;

/// 의미 모델 진단([`namumark_analysis`]) 재노출 — 리다이렉트 후행 내용·향상 제안 등.
pub use namumark_analysis::{Category, Diagnostic, DiagnosticCode, Replacement, Severity, analyze};

/// 나무마크 원문을 의미 모델([`Document`])로 파싱한다.
///
/// 원문 위치·토큰이 필요하면 [`namumark_ast::AstNode::syntax`]로 구문 노드를 쓴다.
pub fn parse(source: &str) -> Document {
    namumark_ast::parse(source)
}
