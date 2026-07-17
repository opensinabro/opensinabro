//! 나무마크 렌더링 pass.
//!
//! ```text
//! Document ──resolve──▶ 특화 IR ──layout──▶ RenderTree
//!            외부 컨텍스트 소비    전역 맥락 확정
//! ```
//!
//! resolve만 외부 세계([`WikiContext`])를 보고 layout은 순수 함수다.
//! 산출물([`namumark_ir::RenderTree`])은 백엔드 크레이트가 소비한다.

mod condition;
mod context;
mod layout;
mod resolve;

pub use context::{Date, DateTime, EmptyContext, Time, WikiContext};

/// resolve + layout을 수행해 백엔드 입력을 만든다.
pub fn build_render_tree(
    document: &namumark_ast::Document,
    context: &dyn WikiContext,
) -> namumark_ir::RenderTree {
    layout::layout(resolve::resolve(document, context))
}
