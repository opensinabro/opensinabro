use crate::kind::SyntaxKind;

/// 파서가 방출하는 이벤트. 트리는 sink가 이벤트를 재생해 만들므로
/// 파서는 이벤트 버퍼 절단만으로 안전하게 백트래킹할 수 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Start { kind: SyntaxKind },
    Finish,
    Token { kind: SyntaxKind, length: u32 },
}
