# 설계 검토: Red-Green Tree 기반 파서 재구축

상태: **구현 완료** (2026-07)

## 구현 노트 (설계와의 편차)

구현 과정에서 다음 세 가지가 설계와 다르게 정착했다. 안전성 목표는 모두 유지된다.

1. **원시 토큰 배열 대신 "불변 원문 + 바이트 길이 이벤트"** — 토큰은 이벤트 버퍼에
   `Token { kind, length }`로 버퍼링되고 sink가 원문을 앞에서부터 재생한다.
   문맥 자유 렉서를 별도로 두는 대신, 검증된 문자열 결정 함수(crate::text)를 그대로
   재사용해 회귀 위험을 최소화했다. 파서의 핵심 불변식은 "방출 위치의 단조 증가"이며
   sink가 길이 총합 == 원문 길이를 assert 한다.
2. **롤백 인프라 미탑재** — 문법 전체가 선결정(문자열로 판단을 끝낸 뒤 방출) 방식으로
   이식되어 snapshot/rollback이 실사용되지 않아 제거했다(미사용 코드 금지 원칙).
   트리 이전 단계 버퍼 구조는 그대로이므로, 추측 파싱이 필요해지면
   `(position, 이벤트 개수)` 복원으로 언제든 재도입할 수 있다.
3. **타입드 접근자 레이어 대신 lowering** — 기존 `ast.rs`(Document)를 의미 모델로
   유지하고 `lower.rs`가 CST에서 이를 생성한다. 공개 API(`parse`)가 불변이라
   기존 테스트 76개와 골든 12건 전부가 파리티 오라클로 작동했다(무변경 통과).
   위치 기반 접근자 레이어는 에디터 기능 단계에서 추가한다.

검증: 이행 기간에 신구 파서 차등 테스트(kitchen sink 전체 접두사·접미사, 문법 조합
768건, 실제 문서 12건)로 동등성을 확인한 뒤 구 파서를 삭제했다. 상시 테스트로
라운드트립 불변식(`tests/lossless_tests.rs`)과 오프셋 → 의미 경로 질의를 유지한다.

## 목표

1. **완전무손실**: `syntax_tree.text() == 원문`이 바이트 단위로 성립. 마커·공백·개행 형식(CRLF)·이스케이프 원문을 전부 보존한다.
2. **위치 기반 의미 질의**: 임의 바이트 오프셋에서 토큰 → 조상 노드 체인을 따라 그 위치의 의미(예: "표 셀 안 → folding 안 → 문서")를 파악할 수 있다.

에디터 기능(미리보기 동기화, 하이라이팅, 자동완성), 소스맵(렌더링 결과 ↔ 원문 스팬), 부분 재파싱, 포맷 보존 편집(봇 편집)의 기반이 된다.

## 현재 구조가 손실하는 것

현재 AST는 해석 결과만 남긴다. 원문 복원이 불가능한 대표 사례:

| 원문 | 현재 AST | 손실 |
|---|---|---|
| `== 제목 ==` | `Heading { level: 2 }` | `=` 마커, 앞뒤 공백 |
| `\|\| 가운데 \|\|` | `alignment: Center` + `"가운데"` | 정렬 공백 자체 |
| `<table align=center>` | `Table/"align"="center"` | 원문 표기(공백 유무) |
| `\[\[문서\]\]` (이스케이프) | `Text("[[문서]]")` | `\` 문자 위치 |
| `1.#42 항목` | `start_number: Some(42)` | 마커 원문 |
| CRLF 개행 | `\n` 기준 lines() | `\r` 소실 |

PROJECT_SCARD 픽스처(14KB) 기준 마커류 등장 약 1,000회 — 문서의 상당 부분이 복원 불가능한 형태로 소비된다. 골든테스트의 residue 지표가 필요했던 이유도 이 손실 때문이다(파싱 성공 여부를 원문 대조로 확인할 수 없음).

## Red-Green Tree 구조 (Roslyn / rust-analyzer 방식)

- **Green tree**: 불변·위치 독립. 각 노드는 `(kind, 길이, 자식들)`, 토큰은 `(kind, 텍스트)`. `Arc` 공유로 동일 서브트리 중복 제거, 증분 재파싱 시 재사용 단위.
- **Red tree** (`SyntaxNode`): green 위의 커서. 부모 포인터와 절대 오프셋을 지연 계산. `node_at_offset(n)`, `ancestors()` 같은 질의를 제공.
- **모든 바이트가 정확히 하나의 토큰에 속한다** → 무손실이 구조적으로 보장된다.

## 설계

### 백트래킹 문제와 해법

rowan의 `GreenNodeBuilder`는 checkpoint로 "이미 방출한 노드를 소급해 감싸기"만 지원하고,
**방출한 토큰을 되돌릴 수 없다**. 나무마크는 추측 파싱이 본질적이다:

- `|| 셀` — 표로 시도하다 첫 행이 미완결이면 문단으로 폴백
- `'''텍스트` — 닫는 마커가 없으면 굵게가 아니라 일반 텍스트
- `[이름]` — 매크로 이름 검증 실패 시 텍스트
- `|캡션|` — 캡션 오인 시 문단

따라서 **트리 빌더에 직접 방출하지 않는다.** rust-analyzer가 rowan 위에서 쓰는 구조를 채택한다:
파서와 트리 구축 사이에 토큰 버퍼와 이벤트 버퍼를 둔다.

### 4단계 파이프라인

```
원문
 │ 1. Lexer (문맥 자유, 한 번만 실행)
 ▼
Vec<RawToken>            ← 토큰 버퍼: (kind, 길이). 불변.
 │ 2. Parser: 커서(usize)로 버퍼 순회, 문법 로직은 기존 파서에서 이식
 ▼
Vec<Event>               ← 이벤트 버퍼: Start(kind) / Finish / Token
 │ 3. Sink: 이벤트 재생 → GreenNodeBuilder (단방향, 역행 없음)
 ▼
SyntaxTree (green/red)
 │ 4. 타입드 AST (공개 API, 현 ast.rs 대체)
 ▼
renderer · 골든테스트
```

**백트래킹은 2단계에서만 일어나며 구조적으로 안전하다**:
파서 상태는 `(커서 인덱스, 이벤트 개수)` 두 정수뿐이므로,

```rust
struct Snapshot { cursor: usize, event_count: usize }

fn snapshot(&self) -> Snapshot;
fn rollback(&mut self, snapshot: Snapshot);   // 커서 복귀 + 이벤트 절단. O(1)
```

트리는 아직 존재하지 않으므로 롤백이 트리를 오염시킬 수 없다.
표 시도 → 실패 → 문단 폴백이 `snapshot/rollback` 한 쌍으로 표현된다.

### 핵심 메커니즘

**렉서 — 문맥 자유 원시 토큰**: 구두점은 1문자 1토큰(`'`, `|`, `{`, `[`, `=`, …),
텍스트·공백은 최장 연속으로 뭉친다. `||`가 표 구분자인지 본문인지는 렉서가 판단하지 않는다
(문맥 의존성이 전부 파서로 이동 → 재렉싱 불필요, 버퍼 불변 유지).

**이벤트와 마커** (rust-analyzer 패턴):

```rust
enum Event {
    Start { kind: SyntaxKind, forward_parent: Option<u32> },  // TOMBSTONE 가능
    Finish,
    Token { kind: SyntaxKind, raw_token_count: u32 },
}
```

- `Marker::complete(kind)` — 노드 종류를 **다 파싱한 뒤에** 확정 (선행 결정 불필요)
- `Marker::abandon()` — 시작한 노드를 무효화(TOMBSTONE), sink가 건너뜀
- `CompletedMarker::precede()` — 이미 완성된 노드를 소급해 감싸기 (`forward_parent`)
- `Token { raw_token_count: 3 }` — 원시 토큰 3개(`'` `'` `'`)를 하나의 `BOLD_MARKER`로 병합

**Sink — 역행 없는 단일 패스**: 이벤트를 순서대로 재생하며 rowan에 방출.
여기서 무손실이 기계적으로 검증된다: 방출한 토큰 길이 총합 ≠ 원문 길이면 즉시 실패.

### 안전 불변식

| 불변식 | 강제 수단 |
|---|---|
| 모든 바이트가 정확히 한 토큰에 속함 | sink에서 길이 총합 == 원문 길이 assert |
| Start/Finish 균형 | Marker가 complete/abandon 없이 drop되면 debug panic |
| 파서 종료 보장 | fuel 카운터(스텝 상한) — 토큰 소비 없는 루프를 즉시 검출, 기존 termination 테스트 유지 |
| 롤백 무결성 | 롤백 대상이 정수 2개뿐 (트리·부수효과 없음) |

### 레이어링과 공개 API

- 공개 API는 타입드 레이어만. rowan 타입은 재노출하지 않는다.
- 렉서·파서·이벤트는 rowan과 무관한 자체 코드다 — sink만 rowan에 접한다.
  기반 교체(자체 green tree, cstree)가 sink 재작성만으로 가능하다.
- `parse(&str) -> SyntaxTree` 시그니처 유지.

### 의미 계산의 위치

트리에는 **구조만** 저장하고, 해석은 타입드 접근자에서 수행한다.

```rust
pub struct TableCell(SyntaxNode);

impl TableCell {
    // 정렬 공백 토큰을 읽어 그 자리에서 계산
    pub fn horizontal_alignment(&self) -> HorizontalAlignment { ... }
    pub fn column_span(&self) -> u32 { ... }   // 선행 `||` 쌍 수 + `<-N>` 토큰
}
```

무손실(토큰 보존)과 의미(접근자)가 한 트리에서 병존한다. 현 파서의 해석 로직(정렬 공백 규칙, colspan, 색상 파싱, 앵커 분리)은 접근자로 이동한다.

### SyntaxKind 초안

```
노드: DOCUMENT, HEADING, PARAGRAPH, QUOTE, LIST, LIST_ITEM, INDENT,
      TABLE, TABLE_ROW, TABLE_CELL, CELL_OPTION,
      CODE_BLOCK, WIKI_STYLE, FOLDING, HTML_BLOCK, COLORED_BLOCK, SIZED_BLOCK,
      BOLD, ITALIC, STRIKETHROUGH, UNDERLINE, SUPERSCRIPT, SUBSCRIPT,
      LITERAL, LINK, IMAGE, CATEGORY, FOOTNOTE, MACRO, COMMENT, REDIRECT
토큰: TEXT, WHITESPACE, NEWLINE, ESCAPED,
      EQUALS_RUN, PIPE_RUN, QUOTE_MARKER, LIST_MARKER, HYPHEN_RUN,
      BRACE_OPEN, BRACE_CLOSE, BRACKET_OPEN, BRACKET_CLOSE, DOUBLE_BRACKET_OPEN, ...
      DIRECTIVE (#!wiki 등), OPTION_TEXT
```

정확한 목록은 구현하며 확정. 원칙: 토큰은 원문 조각의 분류일 뿐 해석을 담지 않는다.

### 파싱 방식의 변화

- `lines()` + 문자열 join 제거 → **토큰 버퍼 인덱스 범위** 기반. "줄"은 NEWLINE 토큰 사이의
  토큰 구간이고, 표 행 수집·문단 세그먼트·셀 분리는 전부 인덱스 범위 연산이 된다
  (문자열 복사 없음, 오프셋은 토큰 길이 누적으로 복원).
- CRLF는 NEWLINE 토큰에 그대로 포함.
- 기존 문법 규칙(openNAMU 대조로 검증된 것)은 그대로 이식한다 — 바뀌는 것은 자료구조이지 문법이 아니다.

### 기존 파서의 추측 지점 → 메커니즘 대응

| 현재 코드 | 새 구조 |
|---|---|
| `parse_table` → None 시 문단 폴백 | snapshot → 표 파싱 → 실패 시 rollback |
| `consume_styled` 닫힘 탐색 실패 | 미리보기(lookahead) 스캔 후 확정, 또는 rollback |
| `parse_brace_block` end==0 → 인라인 | snapshot/rollback |
| 문단 수집 후 세그먼트 분리 | 토큰 구간 재해석 (버퍼 불변이므로 자유) |
| `find_matching_*` 문자열 스캔 | 토큰 버퍼 미리보기 (커서 이동 없는 탐색) |

## 기반 선택

자체 구현은 오프셋·동일성 등 미묘한 버그 위험이 커서 제외. rowan은 위 파이프라인으로
백트래킹 문제가 해소된다 — rowan에 닿는 sink 단계는 역행이 필요 없기 때문이다.

**결론: rowan 채택 + 이벤트 파이프라인.** 렉서·파서·이벤트 버퍼는 자체 코드(rowan 무관)이고,
rowan은 sink 뒤의 저장 형식으로만 쓰인다. 증분 재파싱은 green 서브트리 재사용으로 추후 확장.
(대안 cstree도 동일 구조로 적용 가능 — sink 교체만으로 전환된다)

## 테스트 전략

- **라운드트립 불변식** (신설, 최상위): 모든 픽스처 + termination 코퍼스 입력에 대해 `tree.text() == source`. 무손실의 기계적 증명.
- **골든테스트 유지**: 직렬화를 타입드 레이어에서 수행하고 포맷을 유지 → 이식 중 의미 회귀가 골든 diff로 드러난다. 기존 골든이 이식의 안전망.
- **termination 테스트 유지**: `parse` 시그니처 불변.
- **위치 질의 테스트 신설**: 오프셋 → 의미 경로 (예: 셀 내부 오프셋 → `TABLE_CELL > TABLE_ROW > TABLE > DOCUMENT`).

## 이행 계획

| 단계 | 내용 | 게이트 |
|---|---|---|
| P0 | 렉서 + 이벤트/마커/스냅샷 인프라 + sink(rowan), 문단/텍스트만 파싱 | 전 코퍼스 라운드트립 100% |
| P1 | 블록 문법 이식 (heading·quote·list·indent·hr·comment·redirect) | P0 + 해당 골든 일치 |
| P2 | brace 블록(#!wiki 등) + 인라인 전체 | 〃 |
| P3 | 표 (토큰 구간 기반 행·셀 분리) | 〃 |
| P4 | 타입드 레이어 완성, 골든 전환, 기존 ast.rs 제거 | 전체 76+ 테스트 통과 |

각 단계에서 라운드트립 불변식은 항상 유지. 기존 파서는 P4까지 병존시켜 골든 비교 기준으로 사용한다.
이벤트 파이프라인은 rowan 없이도 단위 테스트 가능하므로(이벤트 스트림 검증) P0에서 렉서·파서 인프라를 독립 검증한다.

## 리스크

- 표 셀 분리·문단 세그먼트를 토큰 구간 연산으로 재작업하는 부분이 가장 어렵다 (현재 문자열 join 기반).
- 트리비아(공백·개행) 소속 규칙을 일관되게 정해야 한다 (rowan 관례: 다음 토큰의 선행 트리비아).
- 이벤트·마커 인프라(~300줄 내외)는 자체 코드다 — 단 rust-analyzer에서 형태가 확립된 패턴이고,
  green tree 오프셋 수학(자체 구현 시의 주 위험)과 달리 정수 버퍼 조작이라 검증이 쉽다.
- 작업량은 파서 내부 전면 재작성 수준. 문법 지식과 테스트 자산은 전부 이월된다.
