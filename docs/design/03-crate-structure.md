# 설계: 단일 역할 크레이트 구조

상태: 구현 완료 (2026-07)

## 목표

크레이트마다 가능한 한 한 가지 역할만 수행한다. AST(의미 모델)와 IR(렌더링 중간 표현)을
독립 크레이트로 분리하고, 공유 유틸의 계층 위반(문자열 판정이 의미 모델 타입을 반환)을 해소한다.

## 구조

```
                    ┌──────────────────┐
                    │  namumark-text   │  표기 판정 유틸 (의존성 0)
                    └───▲──────────▲───┘
                        │          │
┌────────────────┐  ┌───┴──────┐  ┌┴──────────────┐  ┌──────────────┐
│ namumark-ast   │  │ namumark-│  │ namumark-     │  │ namumark-ir  │
│ 의미 모델 타입   │  │ syntax   │  │ parser        │  │ 렌더 IR 타입   │
│ (의존성 0)      │◀─┤ 무손실CST │◀─┤ lowering+parse│  │ +백엔드 계약   │
└────────▲───────┘  │ (rowan)  │  └───────▲───────┘  │ (→ ast)      │
         │          └──────────┘          │          └───▲──────▲───┘
         │          ┌─────────────────────┴──┐            │      │
         └──────────┤ namumark-render        ├────────────┘      │
                    │ resolve/layout pass    │   ┌───────────────┴────────┐
                    │ + WikiContext          │   │ namumark-backend-      │
                    └────────────────────────┘   │ namuwiki (마크업+CSS)   │
                                                 └────────────────────────┘
```

| 크레이트 | 단일 역할 | 의존 | 공개 API |
|---|---|---|---|
| `namumark-text` | 나무마크 표기의 문자열 수준 판정 | 없음 | `heading_shape`, `find_matching_*`, `cell_shape`, `list_marker`, `variable_shape`, … |
| `namumark-ast` | 의미 모델 타입 정의 | 없음 | `Document`, `Block`, `Inline`, … |
| `namumark-syntax` | 완전무손실 구문 트리 | text, rowan | `parse() -> SyntaxTree`, `SyntaxKind`, `SyntaxNode` |
| `namumark-parser` | lowering (구문 트리 → 의미 모델) | syntax, ast, text | `parse() -> Document` |
| `namumark-ir` | 렌더 IR 타입 + 소비 계약 | ast† | `RenderTree`, `RenderBlock`, `Dimension`, `ColorValue`, `RenderBackend` |
| `namumark-render` | resolve·layout pass | ast, ir, parser‡ | `build_render_tree`, `WikiContext`, `Date`/`Time`/`DateTime` |
| `namumark-backend-namuwiki` | 나무위키 동등 마크업 방출 | ast, ir | `NamuwikiMarkup`, `namuwiki_markup()`, `stylesheet()` |

† 표 속성·정렬·리스트 종류는 언어 어휘이므로 ir이 ast 타입을 재사용한다 (초안의 "의존성 0"에서 수정).
‡ render → parser 의존은 `[include]`가 **다른 문서**를 가져와 파싱하기 때문이다.

틀 인자(`@이름@`)와 `{{{#!if}}}`도 이 파이프라인을 그대로 탄다 — 별도 전개 단계를 두지 않는다.
근거는 docs/design/05-template-parameters.md 참고.

## 계층 위반 해소 (text.rs 분해)

기존 `text.rs`는 ast 타입(`ListKind`, `TableAttribute` 등)을 반환해 계층을 가로질렀다.

- **경계 판정**은 `namumark-text`의 자체 어휘로: `ListMarkerKind`, `CellShape`/`CellOption`
  (`CellOptionScope`·`VerticalPosition`·`CellAlignment`). 옵션 유효성 분류까지 text가 담당한다
  — 유효성이 옵션 나열의 끝(경계)을 결정하기 때문이다.
- **의미 매핑**은 `namumark-parser::semantics`로: `ListMarkerKind → ListKind`,
  `CellShape → TableAttribute`들, 이미지 옵션 문자열 → `ImageOption`.
- 소형 enum이 계층별로 중복되는 비용은 "의존성 0 유틸"의 의도된 대가다.

## 픽스처와 테스트 배치

- 실제 문서 픽스처는 워크스페이스 루트 `fixtures/`로 승격 (syntax·parser·backend가 공유).
  생성 코퍼스는 `fixtures/corpus/`로 분리해 골든 스캔에서 제외.
- 테스트: lossless → syntax, 문법·중첩·종료성·AST 골든 → parser,
  pipeline → render, 마크업 검증·마크업 골든 → backend-namuwiki.

## 검증

분리 전후 골든(AST 12건, 마크업 12건)이 재생성 없이 그대로 통과 — 동작 무변경을 기계적으로 확인했다.
