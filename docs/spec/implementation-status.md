# 나무마크 구현 현황 — 누락과 차이

상태: 2026-07. 짝 문서: [나무마크 문법 스펙](namumark.md).

opensinabro 렌더러가 the seed와 **다르게 동작하거나 아직 구현하지 않은** 지점을 모은다.
알파위키 문법 도움말·심화 기준 파리티가 0이므로, 아래가 **알려진 잔여 차이의 사실상 전체**다
(그 밖의 차이는 파리티 정규화가 흡수하거나 아직 발견되지 않은 것이다).

근거 등급 표기는 스펙과 같다: **[렌더확정]**(the seed 렌더 조각 확보) / **[도움말서술]** /
**[미확인]**. 각 항목은 `fixtures/corpus/`의 케이스로 재현된다.

## 분류

- **A. 의도적 누락** — 설계 판단으로 미구현. 대체로 클라이언트·스킨 몫이거나 파리티에
  영향이 없다.
- **B. 미구현 매크로** — the seed는 지원하나 우리는 원문을 그대로 노출한다.
- **C. 미구현 옵션·모드** — the seed는 지원하나 우리는 조용히 폐기하거나 중단한다.
- **D. 버그성 차이** — 우리 쪽 오작동.
- **E. 파싱 한계** — 구조적 제약. 화면 결과는 the seed와 같을 수 있다.
- **F. 우리가 더 관대한 곳** — the seed가 거부하는 것을 우리가 받는다.

---

## A. 의도적 누락 (설계 판단)

### A1. `#!syntax` 구문 강조 (highlight.js) [렌더확정]

the seed는 코드를 highlight.js로 토큰화해 `<span class='hljs-tag'>`… 로 색칠한다. 우리는
클래스·언어까지만 맞추고 **코드를 통짜 텍스트로** 낸다.

```
{{{#!syntax html
<span>x</span>
}}}
→ <pre><code class="hljs" data-language="html"><span>x</span></code></pre>   (토큰 span 없음)
```

하이라이터를 재현하지 않기로 한 판단이다. 파리티 정규화가 `hljs-` span을 벗겨 흡수하므로
파리티 건수에는 잡히지 않는다.

### A2. `[math]` 수식 조판 [미확인]

`[math(식)]`은 MathJax가 읽을 마크업까지만 방출한다. **실제 수식 조판은 클라이언트 MathJax의
몫**이며 the seed도 같은 구조라, 이건 누락이라기보다 렌더 계층의 경계다.

```
[math(x^2)]  → <span class="wiki-math" data-formula="x^2">\(x^2\)</span>
```

### A3. 분류 바 방출 (반대 방향 차이) [렌더확정]

`[[분류:…]]`는 the seed에서 스킨이 그리는 요소라 본문 마크업에 없다. 우리는 자체완결
렌더러라 본문에 `<div class="wiki-categories">`로 낸다. 파리티 정규화는 이 서브트리를
양쪽에서 걷어내 대조한다(`is_dropped_subtree`).

---

## B. 미구현 매크로 (원문 노출)

the seed는 지원하나 우리는 미해결 매크로로 두어 원문을 그대로 방출한다. 화면상 원문이
노출되는 실제 차이다.

| 매크로 | 우리 | the seed | 등급 |
|---|---|---|---|
| `[pagecount]`, `[pagecount(이름공간)]` | `[pagecount]` (원문) | 문서 수 정수 | 렌더확정 |
| `[vimeo(id)]` | `[vimeo(id)]` (원문) | `player.vimeo.com` iframe | 렌더확정 |
| `[navertv(id)]` | `[navertv(id)]` (원문) | `tv.naver.com/embed` iframe | 렌더확정 |

지원하는 영상 매크로는 `[youtube]`·`[kakaotv]`·`[nicovideo]`뿐이다.

---

## C. 미구현 옵션·모드 (폐기 또는 중단)

| 항목 | 입력 | 우리 | the seed | 등급 |
|---|---|---|---|---|
| 표 스코프 색상의 **다크 모드** | `\|\|<bgcolor=#fff,#000> 셀 \|\|` | 라이트만 (`background-color:#fff`) | `data-dark-style`로 다크값 | 렌더확정 |
| `[목차(hide)]` | `[목차(hide)]` | 인자 무시 → 항상 펼침 | 접힌 목차 | 도움말서술 |
| `[youtube]`의 `start=`/`end=` | `[youtube(id,start=8)]` | 크기만, `start`/`end` 폐기 | 지원 (YouTube 전용) | 도움말서술 |
| 이미지 `border-radius=` | `[[파일:x\|border-radius=5]]` | 폐기 | `border-radius: 5px` | 렌더확정 |
| 이미지 `align=middle` | `[[파일:x\|align=middle]]` | 폐기 (정렬 없음) | 가운데(center 동의어) | 도움말서술 |
| 이미지 `rendering=pixelated` | `[[파일:x\|rendering=pixelated]]` | 폐기 | 지원 | 도움말서술 |
| `<color#RRGGBB>` (등호 생략형) | `\|\|<color#ff0000> 글자 \|\|` | 옵션 파싱 중단 → 셀 본문 텍스트 | 글자색 적용 | 도움말서술 |
| 분류 정렬키 | `[[분류:X\|정렬키]]` | 정렬키 폐기 | 색인 정렬키(표시엔 영향 없음) | 도움말서술 |
| 문단명 앵커 | `== 개요 ==` | `s-N` 앵커만 | `<span id='개요'>`를 함께 방출해 `[[#개요]]`가 걸린다 | 렌더확정 |

**표 다크 색상**은 IR에 명시적 후속 과제로 남아 있다([`crates/namumark-ir/src/lib.rs`](../../crates/namumark-ir/src/lib.rs),
`TableStyleProperty`: "색은 듀얼 표기의 라이트 값만 담는다"). 인라인 색(`{{{#fff,#000}}}`)과
`#!wiki dark-style`은 `data-dark-style`을 정상 방출하므로, 표 스코프만 남은 갈래다.

---

## D. 버그성 차이 (오작동)

| 항목 | 입력 | 우리 | the seed | 등급 |
|---|---|---|---|---|
| 분류 `#blur` 오염 | `[[분류:X#blur]]` | 분류명이 `"X#blur"`로 오염 | href에서 `#blur` 분리 + 클래스 | 렌더확정 |
| 빈 문서부 파이프 링크 | `[[\|출력]]` | `wiki-self-link` (볼드 없음) | 현재 문서 링크 + 볼드 | 도움말서술 |

`#blur`는 `lower.rs`가 분류명에서 앵커를 분리하지 않는 버그다.

---

## E. 파싱 한계 (구조적)

### E1. `{{{` 그룹을 블록으로 승격한다 [렌더확정]

나무위키에서 `{{{#!wiki}}}`·`{{{#!if}}}`는 **블록이 아니라 인라인 요소**다. 줄 중간에서
열리고, 독립된 줄에 있어도 그것만 든 문단 안에 들어간다. 우리는 개행이 든 `{{{` 그룹을
무조건 블록으로 승격하므로 문단이 갈라진다.

```text
관련 문서: [[@top1@]]{{{#!if top2 != null
, [[@top2@]]}}}                    → the seed: 한 문단. 우리: 문단이 갈림
```

렌더 증거: the seed에서 `#!wiki` div의 부모가 `<div class='wiki-paragraph'>`인 사례
50건, `#!folding` 8건, `#!syntax` 55건. 문단 안에서 `<br>` 뒤에 `<pre>`가 오는 사례가
33건이다. 최상위에 홀로 선 `#!wiki`도 문단이 감싼다.

**부분 해결**: 각주는 그룹보다 바깥이므로 `emit_paragraph_segments`가 `[*`를 만나면 균형
`]`까지 건너뛰어 각주 범위를 보호한다. `emit_flowing_inline`도 줄머리 마커가 없는 연속
범위면 통째로 인라인 파싱한다(`contiguous_content`). 덕분에 각주 안 여러 줄 블록은 살아났다.

제대로 고치려면 `Inline`이 이들을 품어야 한다(`Inline::WikiStyle`·`Inline::Conditional`·
`Inline::CodeBlock`·`Inline::Folding`). 인라인이 블록을 품는 모양이 되지만(`#!wiki` 안에
표가 들어간다) 나무위키 구조가 실제로 그렇다. 구문 트리도 함께 바뀐다 —
`is_block_boundary`가 `{{{`를 경계로 삼지 않아야 하고, `emit_paragraph_segments`는 문단
노드 **하나**를 열어 그 안에 텍스트와 그룹 노드를 교대로 넣어야 한다.

**주의(실패한 시도)**: 백엔드에서 최상위 `#!wiki`를 `wiki-paragraph`로 감싸 보았으나
파리티가 1,864 → 2,422로 **악화**해 되돌렸다. 문단 중간의 `#!wiki`는 이미 문단 안에 있어야
하는데 우리가 블록으로 떼어 낸 상태라, 거기에 문단을 하나 더 씌우면 문단이 둘로 갈린다.
백엔드만 고쳐선 안 되고 syntax와 함께 가야 한다.

### E2. 그 밖의 한계

- **여러 줄 그룹을 감싼 서식** [미확인] — 서식(`'''` 등)은 줄 단위라, 여러 줄 `{{{` 그룹을
  통째로 감싼 서식은 잔여 텍스트로 남는다. 나무위키에서도 잘 깨지는 관용이다.
- **매크로 이름 오인** [미확인] — 매크로 이름 판정이 `char::is_alphanumeric`이라 `[V]`,
  `[Chorus]`, `[MacBook]` 같은 평범한 대괄호 텍스트가 매크로로 파싱된다. 미해결 매크로는
  원문을 그대로 재구성해 방출하므로 **최종 화면은 the seed와 같고**, AST 의미만 다르다.

---

## F. 우리가 더 관대한 곳

the seed가 거부하는 입력을 우리가 받으면, 나무위키에서 깨져 보일 원문이 우리에게서만
멀쩡히 렌더된다. 파리티 관점에서는 이것도 차이다.

- **`<col bgcolor=>`·`<row bgcolor=>` (공백형)** [도움말서술] — 도움말이 미지원이라 명시하나,
  옵션 이름의 공백을 제거하는 정규화가 `colbgcolor`/`rowbgcolor`로 받아들인다. 공백형이
  허용되는 것은 `table` 계열뿐이 정답이다.

(옛 기록의 "`#transparent`/`#RRGGBBAA` 수용"은 현재는 아니다 — `text::parse_color`가 CSS
색상명 148개 + 3/6자리 hex만 받아 리터럴로 떨어뜨린다. 도움말과 일치한다.)

---

## 검증·근거

각 항목은 `fixtures/corpus/`의 케이스로 재현되고, the seed 실동작은 알파위키 렌더 대조로
확정한다(방법론: [`tools/parity/README.md`](../../tools/parity/README.md)).
파리티 하네스가 새 차이를 발견하면 이 문서에 반영한다.
