# 설계: 틀 인자를 구문 트리로 흡수

상태: 구현 완료 (2026-07). 남은 것은 아래 "`#!if`의 인라인 문맥".

## 배경

`[include(틀:X, 이름=값)]`이 넘긴 인자를 틀 본문의 `@이름@`이 받는다. `@이름=기본값@`으로
기본값을 줄 수 있고, `{{{#!if 조건식}}}`은 조건을 가리는 동시에 **변수를 만든다**.

처음에는 이걸 파싱 **전에** 문자열로 전개했다(별도 크레이트 + 전개 후 재파싱). 근거는
"인자가 마크업 조각을 만든다"였다.

```text
style="display: none;display: @링크=inline;@"     기본값이 CSS 선언을 만드는 것처럼 보인다
@paragraph1=inl@@anchor1=ine@                     두 기본값이 이어져 inline이 된다
```

**이 근거는 틀렸다.** 문법 도움말이 명시한다 — *나무마크 자체 문법엔 매개변수를 쓸 수 없다*
(`<bgcolor=@배경색=#ABCDEF@>` 같은 지정은 불가). 위의 예는 전부 **CSS 값 조각**이지
나무마크가 아니다. 즉 **인자 값은 나무마크 구조를 바꾸지 않는다.**

따라서 값을 몰라도 구조는 확정된다 → 파싱 전에 전개할 이유가 없다. 파이프라인 밖에서
문자열을 전개하고 다시 파싱하던 것은 우회로였다.

## 구조

인자를 red-green tree에 흡수하고 정규 파이프라인을 따른다.

```text
원문 ──syntax──▶ CST ──lower──▶ AST ──resolve──▶ IR
       @이름@ → 토큰              인자를 받아 값 확정
       {{{#!if}}} → Conditional   조건 평가 + 변수 바인딩
```

- 전개용 문자열 전처리와 재파싱이 사라진다.
- `#!if` 조건식 평가기는 `render`가 갖는다 — 외부에서 온 인자를 보는 것은 resolve의 역할이다.
- include가 틀 원문을 파싱하는 것은 남는다. 그건 전개가 아니라 **다른 문서를 가져오는 것**이라 자연스럽다.

## AST 표현

인자가 낄 수 있는 문자열 필드는 `String`이 아니라 조각 리스트(`Template`)다.
변수가 타입으로 드러나고, 문자열 치환이 사라진다.

```rust
pub struct Template(Vec<Fragment>);

pub enum Fragment {
    Text(String),
    Variable(Variable),   // @이름@ / @이름=기본값@
}
```

대상 필드: `WikiStyle::style`·`dark_style`, `Link::target`·`anchor`, `Image::file_name`과
옵션 값, `Macro::argument`, `TableAttribute::value`, `Block::Html`, `Block::Redirect`.
텍스트 문맥의 변수는 `Inline::Variable`이다.

IR부터는 값이 확정되므로 `String` 그대로다 — 백엔드는 영향이 없다. 다만 표 속성은
IR이 AST 타입을 재사용하고 있었으므로(`TableAttribute`), 값이 확정된 `RenderTableAttribute`를
IR에 따로 두었다.

`Template`의 `Display`는 원문 표기(`@이름=기본값@`)를 되살린다 — 값이 정해지기 전 단계의
자연스러운 표현이고, AST 골든이 이걸 쓴다.

인라인 문맥의 인자는 구문 트리가 노드(`TemplateVariable`)로 끊어 준다. 헤더나 옵션처럼
마커 토큰 하나로 들어오는 문자열은 lowering이 `text::variable_shape`로 갈라낸다 —
"leaf 의미는 토큰 텍스트에서 계산한다"는 기존 원칙 그대로다.

## 값 결정

인자 > 기본값 > 빈 문자열. 미치환 변수를 화면에 노출하지 않는다. resolve가 스코프를
들고 있다가 `Template`을 채운다(`fill`). include는 인자를 스코프에 실을 뿐,
원문을 미리 치환하지 않는다 — **파싱은 한 번뿐이다.**

`#!if`가 만든 변수는 뒤따르는 `#!if`와 `@이름@`이 함께 쓴다(`틀:하위 문서`가 `c`·`l`을
앞에서 만들고 뒤에서 재사용한다). 그래서 resolve는 스코프를 순차로 누적한다.

## 남은 것: `{{{` 그룹은 인라인 요소다

나무위키에서 `{{{#!wiki}}}`·`{{{#!if}}}`는 **블록이 아니라 인라인 요소**다. 줄 중간에서
열리고, 독립된 줄에 있어도 그것만 든 문단 안에 들어간다.

```text
관련 문서: [[@top1@]]{{{#!if top2 != null
, [[@top2@]]}}}                                  → 관련 문서: [[문서1]], [[문서2]] (한 문단)
```

렌더 증거: the seed의 `#!wiki` div는 부모가 `<div class='wiki-paragraph'>`다(50건). 최상위에
홀로 선 `#!wiki`도 문단이 감싼다.

우리는 개행이 든 `{{{` 그룹을 무조건 블록으로 승격하므로 문단이 갈라진다. 문자열 전개
방식에서는 이 문제가 우연히 가려져 있었다 — 전개가 끝난 뒤 파싱했기 때문이다.

**부분 해결**(2026-07): 각주는 그룹보다 바깥이므로 `emit_paragraph_segments`가 `[*`를 만나면
균형 `]`까지 건너뛰어 각주 범위를 보호한다. `emit_flowing_inline`도 줄머리 마커가 없는
연속 범위면 통째로 인라인 파싱한다(`contiguous_content`). 덕분에 각주 안 여러 줄 블록이
살아났다.

**남은 것**: `{{{` 그룹 전체가 인라인이다. 렌더 증거로 `#!wiki`(50건)뿐 아니라
`#!folding`(8건)·`#!syntax`(55건)도 부모가 `wiki-paragraph`이고, 문단 안에서 `<br>` 뒤에
`<pre>`가 오는 사례가 33건이다.

그래서 `Inline`이 이들을 품어야 한다 — `Inline::WikiStyle`·`Inline::Conditional`·
`Inline::CodeBlock`·`Inline::Folding`. 인라인이 블록을 품는 모양이 되지만(`#!wiki` 안에
표가 들어간다), 나무위키 구조가 실제로 그렇다.

구문 트리 쪽도 함께 바뀐다. `is_block_boundary`가 `{{{`를 경계로 삼지 않아야 하고,
`emit_paragraph_segments`는 문단 노드 **하나**를 열어 그 안에 텍스트와 그룹 노드를
교대로 넣어야 한다(지금은 그룹마다 문단이 갈린다).

**주의(실패한 시도)**: 백엔드에서 최상위 `#!wiki`를 `wiki-paragraph`로 감싸 보았으나
1,864 → 2,422로 **악화**해 되돌렸다. 문단 중간의 `#!wiki`는 이미 문단 안에 있어야 하는데
우리가 블록으로 떼어 낸 상태라, 거기에 문단을 하나 더 씌우면 문단이 둘로 갈린다.
백엔드만 고쳐선 안 되고 syntax와 함께 가야 한다.

## 조건식 언어

나무위키 틀에서 관찰된 범위만 다룬다(`render`의 `condition`).

```text
시퀀스   a, b            (마지막 값이 결과)
대입     이름 = 식
삼항     조건 ? 참 : 거짓
논리     a || b   a & b   (틀은 논리 AND에 단일 앰퍼샌드를 쓴다)
비교     a == b   a != b
덧셈     a + b           (문자열 연결)
후위     .length   .startsWith(x)   .substr(a[, b])   .lastIndexOf(x)   ?.
기본     null   '문자열'   "문자열"   숫자   이름   ( 식 )
```

`calleeTitle`(호출한 문서 제목)은 `WikiContext::current_title()`이 공급한다.
truthy 규칙은 JavaScript를 따른다(`null`·빈 문자열·0·false가 거짓).
