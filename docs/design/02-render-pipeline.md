# 설계: 렌더링 파이프라인 (다단계 lowering)

상태: 1차 구현 완료 (2026-07)

구현 노트: `resolve`가 분류 수집과 문단 분할(블록 성격 매크로 승격)도 담당한다.
`[date]`/`[datetime]`은 `WikiContext::now()`로 해석된다. IR의 크기·색상은
`Dimension`(Pixels/Percentage/Custom)·`ColorValue`(Named/Rgb)로 특화되어 있다.
나무위키 동등 마크업 백엔드(`NamuwikiMarkup`, `namuwiki_markup()`)는 IR 타입마다
대응하는 마크업 래퍼(BlockMarkup, InlineMarkup, ...)가 `std::fmt::Display`를 구현하는
합성 구조이며, 태그 방출은 스택 전용 스트리밍 태그 라이터(`tag::Tag`)를 거친다 —
속성값 이스케이프 자동 적용, 여닫이 짝 구조적 보장, 힙 할당 없음. 유일한 예외는
형제 블록에 걸치는 헤딩 콘텐츠 래퍼로 문서 래퍼가 수동 관리한다. 자원 회귀는
`allocation_probe` 테스트가 게이트한다(할당 횟수 기준선 + peak 상한 3×출력).
이스케이프도 무할당 Display 어댑터로 수행된다. IR 노드는 자기완결적이다
(각주 라벨 내장, FootnoteSection·TableOfContents가 내용 소유). `#!html` sanitizer와 표 듀얼 색상의 다크 값 적용은 후속 과제
(sanitizer 도입 전까지 이스케이프된 코드 박스로 방출, 안전 우선).

## 목표

여러 렌더링 백엔드를 지원한다. 1차 백엔드는 나무위키 동급 마크업(HTML).
rustc가 HIR → THIR → MIR로 내리며 시맨틱을 특화하듯, 백엔드 앞에 전처리 pass를
두어 맥락을 단계적으로 수집·확정한다. 백엔드는 마지막 IR을 순회만 하는 얇은 층이다.

## 파이프라인

```
CST ──lower──▶ Document ──resolve──▶ ResolvedDocument ──layout──▶ RenderTree ──▶ 백엔드들
(무손실)        (구현됨)               (외부 컨텍스트 소비)           (전역 맥락 확정)    HTML | ...
```

| IR | 확정되는 것 | 아직 모르는 것 |
|---|---|---|
| Document | 문법 구조. 문서-지역적, 컨텍스트 프리 | 매크로 의미, 링크 존재, 각주 번호 |
| ResolvedDocument | 매크로 특화(`Age`/`Video`/`Ruby`/`Math`/TOC·각주 자리표시), 링크 해석(존재 여부, 내부/외부/이미지 URL), include 확장(인자 치환, 순환·깊이 방어) | 헤딩·각주 번호, TOC 내용 |
| RenderTree | 섹션 트리(`s-1.2` 앵커, 접기), 각주 번호·이름 병합·`[각주]` 위치 방출·문서 끝 잔여 방출, TOC 실체화, 분류 분리 수집 | 없음 |

pass 순서 = 의존성: include 확장이 새 각주·분류·헤딩을 가져오므로
resolve가 번호 부여(layout)보다 먼저다. 번호 부여는 트리 확정 후 1회.

## 외부 컨텍스트

resolve pass만 외부 세계를 본다. layout과 백엔드는 순수 함수다.

```rust
pub trait WikiContext {
    fn document_exists(&self, title: &str) -> bool;          // 빨간 링크
    fn include_source(&self, title: &str) -> Option<String>; // [include] 원문
    fn file_url(&self, file_name: &str) -> Option<String>;
}
```

- 기본 no-op 구현을 제공해 컨텍스트 없이도 전 파이프라인이 동작한다.
- 미지원 매크로는 `Unresolved { name, argument }`로 보존하고 백엔드가 원문 표기로
  방출한다 (화면 일치 원칙).

## 백엔드

```rust
pub trait RenderBackend {
    type Output;
    fn render(&self, document: &RenderTree) -> Self::Output;
}
```

1차: 나무위키 동급 마크업 + 자체 CSS 동봉. 듀얼 색상(`#fff,#000`)은
CSS 변수 + `.dark` 클래스. `#!html`과 style 값은 allowlist sanitize.
이후 백엔드(plain text, Markdown, 검색 색인 텍스트)는 RenderTree 순회만 구현한다.

## 배치와 검증

- 새 크레이트 `crates/namumark-render`: IR 2종 + resolve/layout pass + `backend/html.rs`.
  백엔드가 늘어나면 그때 크레이트를 분리한다.
- 검증: 픽스처 12건의 HTML 골든 스냅샷(기존 골든 방식 연장) + 실제 나무위키 화면 대조.
  중간 IR도 동일한 직렬화 방식으로 골든화할 수 있다.
