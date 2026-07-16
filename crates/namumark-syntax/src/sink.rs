use crate::event::Event;
use crate::kind::{NamumarkLanguage, SyntaxKind};
use rowan::{GreenNode, GreenNodeBuilder, Language};

/// 이벤트를 순서대로 재생해 green tree를 만든다. 여기서는 역행이 없다.
///
/// 무손실 검증: 토큰 길이 총합이 원문 길이와 다르면 파서 버그이므로 즉시 실패한다.
pub(crate) fn build_tree(source: &str, events: &[Event]) -> GreenNode {
    let mut builder = GreenNodeBuilder::new();
    let mut offset = 0usize;
    for event in events {
        match *event {
            Event::Start {
                kind: SyntaxKind::Tombstone,
            } => {}
            Event::Start { kind } => builder.start_node(NamumarkLanguage::kind_to_raw(kind)),
            Event::Finish => builder.finish_node(),
            Event::Token { kind, length } => {
                let end = offset + length as usize;
                builder.token(NamumarkLanguage::kind_to_raw(kind), &source[offset..end]);
                offset = end;
            }
        }
    }
    assert_eq!(offset, source.len(), "무손실 위반: 원문을 전부 덮지 못했다");
    builder.finish()
}
