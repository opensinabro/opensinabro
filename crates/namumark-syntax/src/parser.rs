use crate::event::Event;
use crate::kind::SyntaxKind;

/// 이벤트 방출 파서.
///
/// 핵심 불변식: 토큰 이벤트는 원문을 앞에서부터 빈틈없이 덮는다(`position`이 단조 증가).
/// 따라서 sink의 길이 검증과 함께 무손실이 구조적으로 보장된다.
///
/// 트리는 sink가 이벤트를 재생해 만들므로 파서 단계의 재해석·추측은 트리를
/// 오염시킬 수 없다. 현 문법은 전부 선결정(결정 후 방출)이라 롤백이 필요 없지만,
/// 필요해지면 `(position, 이벤트 개수)` 복원으로 재도입할 수 있다.
pub(crate) struct Parser<'source> {
    source: &'source str,
    position: usize,
    events: Vec<Event>,
    fuel: usize,
}

/// 시작했지만 아직 종류가 정해지지 않은 노드. 파싱을 마친 뒤 `complete`로 확정한다.
pub(crate) struct Marker {
    event_index: usize,
    finished: bool,
}

impl<'source> Parser<'source> {
    pub(crate) fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
            events: Vec::new(),
            // 소비 없는 루프 방어용 상한. 정상 파싱은 바이트당 몇 이벤트 수준이다.
            fuel: 256 + source.len() * 32,
        }
    }

    pub(crate) fn source(&self) -> &'source str {
        self.source
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn finish(self) -> Vec<Event> {
        self.events
    }

    /// 연료가 남아 있으면 true. 문법 루프는 false를 받으면
    /// 잔여 원문을 구조 없이 방출하고 중단한다 (소비 없는 루프 방어).
    pub(crate) fn tick(&mut self) -> bool {
        if self.fuel == 0 {
            return false;
        }
        self.fuel -= 1;
        true
    }

    pub(crate) fn start_node(&mut self) -> Marker {
        let event_index = self.events.len();
        self.events.push(Event::Start {
            kind: SyntaxKind::Tombstone,
        });
        Marker {
            event_index,
            finished: false,
        }
    }

    /// `end`까지의 원문을 `kind` 토큰 하나로 방출한다. 시작점은 항상 현재 위치다.
    pub(crate) fn emit_token(&mut self, kind: SyntaxKind, end: usize) {
        debug_assert!(end >= self.position, "토큰 방출이 역행했다");
        debug_assert!(self.source.is_char_boundary(end), "문자 경계가 아니다");
        if end == self.position {
            return;
        }
        let length = (end - self.position) as u32;
        self.events.push(Event::Token { kind, length });
        self.position = end;
    }

    pub(crate) fn events_mut(&mut self) -> &mut Vec<Event> {
        &mut self.events
    }
}

impl Marker {
    pub(crate) fn complete(mut self, parser: &mut Parser<'_>, kind: SyntaxKind) {
        debug_assert!(!self.finished);
        self.finished = true;
        let Event::Start { kind: slot } = &mut parser.events_mut()[self.event_index] else {
            unreachable!("marker는 Start 이벤트를 가리킨다");
        };
        *slot = kind;
        parser.events_mut().push(Event::Finish);
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        debug_assert!(
            self.finished || std::thread::panicking(),
            "Marker가 complete 없이 버려졌다"
        );
    }
}
