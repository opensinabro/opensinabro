# 나무마크 문법 스펙

상태: 초판 (2026-07). 대상: 구현자.

나무위키 엔진(the seed)이 쓰는 마크업 언어 **나무마크**의 문법을 구현 가능한 수준으로
규명한 문서다. the seed는 비공개라 공식 스펙이 없으므로, 이 문서의 모든 규칙은 아래
**근거 체계**에 따라 대조실험으로 도출한 것이다. opensinabro 구현이 이 규칙을 따르며,
각 예시의 렌더 결과는 저장소의 골든 테스트(`fixtures/corpus/`)가 검증한다.

이 문서는 공익 목적의 투명한 규명이다. 본가가 공개하지 않는 문법을 재현 가능한 실험으로
밝혀 생태계가 공유하도록 한다.

---

## 0. 읽는 법

### 0.1 근거 체계

같은 문법이라도 근거마다 신뢰도가 다르다. 판단이 갈리면 위쪽을 따른다.

| 순위 | 근거 | 성격 |
|---|---|---|
| 1 | the seed 엔진의 실제 렌더 결과 | 확정 |
| 2 | 나무위키/알파위키 문법 도움말 서술 | 스펙 서술 |
| 3 | 실제 문서 덤프의 사용 양상 | 빈도·실재성 |
| 4 | openNAMU 등 타 구현 소스 | 참고 (자체 확장 섞임) |

the seed 본체는 접근이 막혀 있어(namu.wiki는 크롤링 차단), **알파위키**(`www.alphawiki.org`)를
1차 근거로 삼는다. 알파위키는 the seed와 같은 엔진을 쓰며 원문(`GET /raw/<문서>`)과 렌더
결과(`GET /w/<문서>`)를 모두 공개한다. 알파위키의 `알파위키:문법 도움말`·`/심화`는 나무위키
문법 도움말을 그대로 옮긴 문서라, 그 원문과 렌더 결과를 대조하면 the seed 실동작이 확정된다.
자세한 방법은 [`tools/parity/README.md`](../../tools/parity/README.md).

각 규칙에는 근거 등급을 붙인다.

- **[렌더확정]** — the seed 렌더 HTML에서 대응 출력을 실제로 확인함.
- **[도움말예제]** — 문법 도움말에 원문 예제가 실려 있어 렌더 대조가 가능함.
- **[도움말서술]** — 도움말이 서술하지만 그 마크업 자체는 예제로 실려 있지 않음.
- **[미확인]** — 도움말에 근거가 없어 실측 경로가 없음. 구현의 현 동작을 기록한 것.

**서술과 실동작이 어긋나는 사례가 있다**(예: 도움말은 `image-rendering`이 동작하는 것처럼
서술하지만 the seed는 제거한다). 등급이 [도움말서술]뿐인 규칙은 구현 전 렌더로 재확인한다.

### 0.2 표기 규약

예시는 `입력` → `렌더 HTML`로 적는다. 렌더 결과는 다음 고정 문맥에서 얻은 것이다
(`fixtures/corpus`의 `CorpusContext`).

- 현재 문서명: `시험 문서/하위`
- 존재하는 문서: `문서`, `알파위키` 등 (그 외는 "없는 문서" — 빨간 링크)
- 올라온 파일: `example.png`, `알파위키 로고.svg`
- 고정 시각: `2026-07-17 12:00:00`

렌더 클래스 어휘(`wiki-table`, `wiki-paragraph`, `wiki-macro-toc` 등)는 the seed와 **같다**.
색상 표기(`#ff0000` vs `rgb(...)`)나 style 선언 순서 같은 표현 차이는 the seed와 다를 수
있으나, 구조(태그·중첩·style 시맨틱)는 일치한다.

### 0.3 이 문서가 다루지 않는 것

- 위키 UI(편집 링크, 분류 바 등 스킨이 그리는 것). 본문 마크업 산출물만 다룬다.
- 렌더 파이프라인·크레이트 구조. 그건 각 크레이트의 문서주석과 루트 README의 몫이다.
- 임의 마크업의 the seed 실측. 알파위키에 임의 입력을 넣는 읽기 전용 경로가 없어,
  도움말 예제나 실제 문서가 쓰지 않는 패턴은 [미확인]으로 남는다.
- **opensinabro 구현이 the seed와 다르거나 아직 구현하지 않은 지점** — 이 문서는 the seed의
  문법을 서술한다. 우리 구현의 누락·차이는 [구현 현황 문서](implementation-status.md)에 있다.

---

## 1. 문서 모델

### 1.1 블록과 인라인

나무마크 문서는 **블록**의 나열이고, 블록 안에 **인라인**이 흐른다.

- 블록: 헤딩, 문단, 수평줄, 인용문, 리스트, 들여쓰기, 표, 주석, 리다이렉트.
- 인라인: 텍스트, 서식(굵게 등), 링크, 이미지, 각주, 매크로, 색상·크기, 중괄호 그룹, 틀 인자.

의미 모델의 최상위 타입은 `Block`과 `Inline`이다
([`crates/namumark-ast/src/lib.rs`](../../crates/namumark-ast/src/lib.rs)).

한 가지 중요한 비대칭: **중괄호 그룹(`{{{ … }}}`)은 인라인 요소다.** `#!wiki`·`#!folding`·
`#!syntax`·`#!html`·`#!if`·색상·크기 그룹은 모두 인라인이며, 그 안에 표 같은 블록을
품을 수 있다(인라인이 블록을 감싸는 모양). 이것이 문단 중간에서 `{{{#!wiki`가 열려도
문단이 갈라지지 않는 이유다. → [12장](#12-중괄호-그룹)

### 1.2 문단과 빈 줄 — 빈 줄은 문단 경계가 아니다

**[렌더확정]** 나무위키는 빈 줄로 문단을 나누지 않는다. 빈 줄은 `<br>`이고, 문단(`wiki-paragraph`)의
경계는 **헤딩**이 만드는 구역이다.

```
앞

뒤
```
→ `<div class="wiki-paragraph">앞<br><br>뒤</div>`

문단 안의 개행은 원칙적으로 모두 `<br>`이다. 예외는 아래뿐이다.

- **분류·`[include]`만 있는 줄**은 개행까지 통째로 사라진다(→ [13.3](#133-분류)).
- **`##` 주석 줄**도 개행까지 사라진다(→ [13.1](#131-주석)).
- 연속된 빈 줄은 빈 문단 하나로 합쳐 `(줄 수 − 1)`개의 `<br>`를 담는다. the seed는 빈 문단을
  절대 연달아 내지 않는다.

### 1.3 이스케이프 — `\`

**[도움말예제]** 역슬래시는 바로 뒤 한 글자의 문법적 의미를 없앤다. 인라인·블록 어느
문법이든 앞에 `\`를 붙이면 글자로 취급된다.

```
\== 문단 문법 무효 ==   → == 문단 문법 무효 ==
\--취소선 무효--        → --취소선 무효--
\{{{#red 색상 무효}}}   → {{{#red 색상 무효}}}
\|| 표 문법 무효 \||    → || 표 문법 무효 ||
\\                      → \
```

링크 본문·표 셀 구분처럼 인라인 파서를 거치지 않는 자리에서도 `\`는 존중된다:
`\|`는 링크·셀 구분자가 아니고, `\#`는 앵커 구분자가 아니며, `\]`는 링크 닫음이 아니다
(→ [4장](#4-링크), [9장](#9-표)).

### 1.4 수평줄

**[도움말예제]** 하이픈만으로 이뤄진 줄. **정확히 4~9개**일 때만 `<hr>`이고, 그 밖은 문단이다.

```
----   ~   ---------   (4~9개)  → <hr>
---                    (3개)    → 글자 그대로
----------             (10개)   → 글자 그대로
---- 뒤                (뒤 글자) → 글자 그대로
```

`<hr>`에는 클래스가 없다.

### 1.5 본문의 HTML 특수문자

**[미확인]** 본문 텍스트의 `<`·`>`·`&`는 HTML 엔티티로 이스케이프된다(`"`는 본문에서는 그대로 둔다 — 속성값에서만 이스케이프).

```
<script>alert(1)</script> & "따옴표"
→ &lt;script&gt;alert(1)&lt;/script&gt; &amp; "따옴표"
```

위키 입력이 원시 HTML로 나가는 유일한 통로는 `#!html`이며, 그마저 sanitizer를 거친다
(→ [12.5](#125-html--원시-html)).

---

## 2. 인라인 서식

**[도움말예제]** 서식 마커는 내용을 같은 마커로 감싼다.

| 문법 | 결과 | 태그 |
|---|---|---|
| `'''굵게'''` | 굵게 | `<strong>` |
| `''기울임''` | 기울임 | `<em>` |
| `~~취소선~~` 또는 `--취소선--` | 취소선 | `<del>` |
| `__밑줄__` | 밑줄 | `<u>` |
| `^^위첨자^^` | 위첨자 | `<sup>` |
| `,,아래첨자,,` | 아래첨자 | `<sub>` |

```
'''굵게'''  → <div class="wiki-paragraph"><strong>굵게</strong></div>
,,아래첨자,, → <div class="wiki-paragraph"><sub>아래첨자</sub></div>
```

- **중첩** [미확인]: 서로 다른 서식은 중첩된다. `'''굵게 __밑줄__ 굵게'''` →
  `<strong>굵게 <u>밑줄</u> 굵게</strong>`.
- **마커 인접** [미확인]: 마커가 한글·글자에 바로 붙어도 된다. `앞'''굵게'''뒤` →
  `앞<strong>굵게</strong>뒤`.
- **취소선 혼용 불가** [도움말예제]: `--`와 `~~`를 섞으면 마커로 인식되지 않는다.
  `~~취소선--` → 글자 그대로.
- **여러 줄** [미확인]: 한 문단 안이면 서식이 개행을 넘을 수 있다.
  `'''여러\n줄 굵게'''` → `<strong>여러<br>줄 굵게</strong>`.
  단, 서식이 여러 줄 `{{{ }}}` 그룹을 감싸면 줄 단위 처리의 한계로 마커가 잔여 텍스트로 남는다.

### 2.1 깨진 서식

**[미확인]** 나무위키에서도 깨져 보이는 입력은 우리도 깨진 채 재현한다.

- **미닫힘**: 닫는 마커가 없으면 여는 마커도 글자다. `'''닫히지 않은 굵게` → 글자 그대로.
- **교차**: 마커가 서로 엇갈리면 안쪽이 성립하지 않는다.
  `'''굵게 ''기울임''' 기울임''` → `<strong>굵게 ''기울임</strong> 기울임''`.
- **빈 마커**: `''''''`(따옴표 6개) → 글자 그대로.

---

## 3. 글자 크기와 색상

### 3.1 크기 — `{{{+N …}}}` / `{{{-N …}}}`

**[도움말예제]** `+`/`-` 뒤 `1`~`5` 한 자리, 그 뒤 공백 하나, 그리고 내용.

```
{{{+1 +1단계}}}  → <span class="wiki-size-up-1">+1단계</span>
{{{-3 -3단계}}}  → <span class="wiki-size-down-3">-3단계</span>
```

- 단계는 `±1`~`±5`. 범위를 벗어나면 크기가 아니라 리터럴이다:
  `{{{+6 범위밖}}}` → `<code>+6 범위밖</code>` [미확인].
- **중첩은 클래스가 겹쳐 누적**된다 [도움말예제]:
  `{{{+5 {{{+5 +10단계}}}}}}` → `<span class="wiki-size-up-5"><span class="wiki-size-up-5">…</span></span>`.
- 크기 표기(`+N`) 뒤에는 내용을 가르는 **공백이 반드시** 있어야 한다
  (`parse_size_marker`).

### 3.2 색상 — `{{{#색상 …}}}`

**[도움말예제]** `#` 뒤 색상 표기, 공백 하나, 내용.

```
{{{#red 텍스트}}}    → <span style="color:red" data-dark-style="color:red;">텍스트</span>
{{{#ff0000 텍스트}}} → <span style="color:#ff0000" data-dark-style="color:#ff0000;">텍스트</span>
```

**유효한 색상 표기:**

- **3자리 / 6자리 hex**: `#f00`, `#ff0000`. 3자리는 6자리로 정규화된다(`#f00`→`#ff0000`).
  대소문자 무관, 소문자로 정규화된다(`#FF0000`→`#ff0000`) [도움말서술].
- **CSS 색상명 148개** [렌더확정]: `red`, `blue` 등 정식 CSS 색상 이름만
  (목록 → [부록 A](#부록-a-css-색상명-148개)). 이름은 `#`을 붙여도 되고 안 붙여도 된다.

**색상이 아니라 리터럴이 되는 경우** [렌더확정]:

- **표기 뒤 공백이 없으면** 색상이 아니다: `{{{#212529}}}` → `<code>#212529</code>`.
  이 덕에 `{{{#redirect 목적지 문서}}}`처럼 `#`으로 시작하는 리터럴이 색상으로 오인되지 않는다
  (`redirect`는 148개 목록에 없다).
- **148개 목록·hex가 아닌 이름**: `{{{#transparent 투명}}}` → `<code>#transparent 투명</code>`,
  `{{{#ff000080 반투명}}}`(`#RRGGBBAA`) → `<code>…</code>`. 도움말도 `#transparent`·`#RRGGBBAA`
  미지원이라 서술한다.

**라이트/다크 쌍** [도움말예제]: `,`로 두 색을 잇는다. **쉼표 뒤에 공백이 있으면 무효**다.

```
{{{#888,#ff0 다크테스트}}}  → color:#888888 · data-dark-style:color:#ffff00;
{{{#888, #ff0 공백}}}       → <code>#888, #ff0 공백</code>   (무효 → 리터럴)
```

다크 색을 지정하지 않으면 `data-dark-style`은 라이트와 같은 값으로 채워진다(위 예시).

### 3.3 크기·색상 중첩

**[도움말예제]** 크기와 색상은 순서에 무관하게 중첩된다.

```
{{{+1 {{{#blue 큰글자파랑}}}}}}
→ <span class="wiki-size-up-1"><span style="color:blue" …>큰글자파랑</span></span>
```

색상 안팎으로 다른 서식을 둘 수 있고, 서식이 색상을 감싸는지 여부에 따라 적용 범위가 갈린다:

```
{{{#red __밑줄 포함__}}}  → <span …color:red…><u>밑줄 포함</u></span>   (밑줄에 색 적용)
__{{{#red 밑줄 제외}}}__  → <u><span …color:red…>밑줄 제외</span></u>   (밑줄엔 색 미적용)
```

---

## 4. 링크

`[[대상]]`, `[[대상|표시]]`, `[[대상#앵커|표시]]`. 대상·표시는 `\|` 이스케이프를 존중해
가른다(`split_link_body`). 앵커는 **마지막** `\`-없는 `#`으로 가른다(`split_anchor`).

### 4.1 문서 링크

**[도움말예제]**

```
[[문서]]           → <a class="wiki-link-internal" href="/w/%EB%AC%B8%EC%84%9C" title="문서">문서</a>
[[문서|출력글자]]  → <a class="wiki-link-internal" href="/w/%EB%AC%B8%EC%84%9C" title="문서">출력글자</a>
```

**링크 종류**는 클래스로 구분된다 [렌더확정]:

| 종류 | 클래스 | rel |
|---|---|---|
| 존재하는 문서 | `wiki-link-internal` | 없음 |
| 없는 문서 | `wiki-link-internal not-exist` | `nofollow` |
| 현재 문서·빈 대상 | `wiki-self-link` | — |

```
[[문서]] [[없는 문서]]
→ …class="wiki-link-internal"…문서…  …class="wiki-link-internal not-exist" rel="nofollow"…없는 문서…
```

### 4.2 앵커

**[도움말예제]** `#`으로 현재 문서의 문단(번호 `s-N` 또는 문단명)에 링크한다.

```
[[#s-1|1문단으로]]  → <a class="wiki-link-internal" href="#s-1" title="">1문단으로</a>
[[#개요]]           → <a class="wiki-link-internal" href="#%EA%B0%9C%EC%9A%94" title="">#개요</a>
[[알파위키#s-6|참고]] → href="/w/%EC%95%8C…#s-6" title="알파위키"
```

- 표시부가 없으면 **적힌 그대로가 글자**다: `[[#개요]]` → 표시 `#개요`, `[[/심화]]` → `/심화`.
  단 **다른 문서의 앵커는 표시에서 빠진다**: `[[알파위키#기능]]` → 표시 `알파위키`.
- 마지막 `#`은 뒤가 비어도 구분자다: `[[##]]` → 제목 `#`, 앵커 없음 [미확인].

### 4.3 상대 링크 / 이름공간

**[도움말예제/서술]**

- `[[../]]` → 상위 문서(없으면 자기 자신). `[[/하위]]` → 현재 문서의 하위. 여러 번 써도
  한 단계만 올라간다: `[[../../]]`는 상위의 상위로 가지 않는다 [도움말서술].
- `[[:분류:…]]`, `[[:파일:…]]` — 콜론 접두사로 분류·파일을 **텍스트 링크**로 만든다
  (분류 등록·이미지 삽입이 아니라):
  `[[:분류:분류]]` → `<a … href="/w/%EB%B6%84%EB%A5%98:%EB%B6%84%EB%A5%98" …>분류:분류</a>`.
- `[[문서:…]]` — `문서:` 접두사는 이름공간이 아니라 "본문 이름공간 못박기"다. 제목이 `/`로
  시작해 하위 문서로 읽히는 걸 막고, 접두사 자체는 떨어져 나간다 [렌더확정].

### 4.4 외부 링크

**[도움말예제]** `http://`, `https://`, `ftp://`로 시작하는 대상.

```
[[https://www.google.com/|구글]]
→ <a class="wiki-link-external" href="https://www.google.com/" target="_blank"
     rel="nofollow noopener ugc" title="https://www.google.com/">구글</a>
```

외부 링크의 `title`은 URL이되 `#` 뒤는 뺀다 [렌더확정]. 앵커 분리(`#`)도 외부 URL에는
적용하지 않는다(URL의 `#`은 프래그먼트).

### 4.5 이스케이프·미닫힘

**[도움말예제/미확인]**

```
[[문서\|표시]]      → 제목 "문서|표시" (\| 는 구분자 아님)
[[\#1 to Infinity]] → 제목 "#1 to Infinity"
[[[12:00\]]]        → 제목 "[12:00]" (\] 는 닫음 아님)
[[문서              → 글자 그대로 (닫히지 않은 링크)
```

### 4.6 퍼센트 인코딩

**[렌더확정]** 인코딩 방식이 자리마다 다르다.

- **문서 경로**: 대문자 hex. `:`·`/`·`(`·`)`는 인코딩하지 않고 그대로 둔다. 공백은 `%20`.
  `[[표(자료)]]` → `/w/%ED%91%9C(%EC%9E%90%EB%A3%8C)`.
- **각주 앵커**: 소문자 hex, 전부 인코딩. `href="#fn-%ec%98%88%ec%8b%9c"`.
- **분류 푸터의 링크**: 콜론을 `%3A`로 인코딩(문서 링크의 `:` 미인코딩과 다르다).

---

## 5. 이미지

`[[파일:이름|옵션&옵션&…]]`. 옵션은 `&`로 잇고 `이름=값` 꼴이다.

### 5.1 기본

**[도움말예제]**

```
[[파일:example.png]]
→ <span class="wiki-image-align-normal" style="">
    <span class="wiki-image-wrapper" style="width: 100%;">
      <img width="100%" src="/file/example.png" alt="파일:example.png"></span></span>
```

- **올라오지 않은 파일**은 이미지가 아니라 링크로 떨어진다 [미확인]:
  `[[파일:없는 파일.png]]` → `<a class="wiki-link-internal not-exist" …>파일:없는 파일.png</a>`.
- **파일명 앞 공백**이 있으면 파일 문법이 아니라 일반 링크다 [도움말예제].
- `file:`(영문 접두사)도 파일로 인식된다 [미확인].
- **링크 건 이미지** [도움말예제]: `[[대상|[[파일:…]]]]` — 표시부에 이미지를 넣는다.

### 5.2 옵션

| 옵션 | 결과 | 등급 |
|---|---|---|
| `width=100` | `width: 100px` (단위 없는 정수 → px) | 도움말서술 |
| `width=100%` | `width: 100%` | 도움말예제 |
| `width=21.6px` | `width: 21px` (**소수는 정수로 절삭**) | 미확인 |
| `height=50` | `height: 50px` | 도움말서술 |
| `align=left/center/right` | `wiki-image-align-…` 클래스 | 도움말서술 |
| `bgcolor=#ff0000` | wrapper에 `background-color` | 도움말서술 |
| `theme=dark` | `wiki-image-theme-dark` 클래스 | 도움말서술 |

이미지 크기의 소수는 **정수 px로 절삭**한다(표 셀의 `<width=33.3>`이 `33.3`을 그대로 두는
것과 다르다 → [9장](#9-표)).

**미지원·폐기되는 옵션** [렌더확정/서술]: 모르는 옵션(`우주=1`)은 조용히 폐기.
`align=middle`(도움말은 center 동의어), `border-radius=`, `rendering=pixelated`는 도움말이
지원이라 하나 우리는 폐기한다(the seed 실동작 재확인 필요).

---

## 6. 헤딩

**[도움말예제]** 줄 양끝을 같은 개수의 `=`로 감싼다. 수준은 `=` 개수(1~6). 안쪽에 공백이
있어야 하고, 바깥에 다른 글자가 있으면 안 된다.

```
== 문단 2 ==
→ <h2 class="wiki-heading"><a id="s-1" href="#toc">1.</a> <span id="문단 2">문단 2</span></h2>
  <div class="wiki-heading-content"></div>
```

- 수준: `= … =`(h1)부터 `====== … ======`(h6). **7개 이상은 헤딩이 아니다** → 문단.
  1단계는 도움말상 일반적으로 금지지만 렌더는 된다.
- **미출력 조건** [도움말예제]:
  - 안쪽 공백 없음: `==문단==` → 문단.
  - 양옆 `=` 개수 불일치: `== 문단 ===` → 문단.
  - 바깥에 글자: `== 문단 == `(뒤 공백) → 문단.
- **접힌 문단** [도움말예제]: `==# 문단 #==` → `<h2 class="wiki-heading wiki-heading-folded">…`.
  마커가 `#`을 안팎에 하나씩 더 가진다.

### 6.1 번호·앵커·구역

**[렌더확정/미확인]**

- 문단 번호는 `s-1`, `s-1.1`, `s-2` …로 매긴다. 링크는 `<a id="s-N" href="#toc">N.</a>`
  (번호 뒤 점이 링크 **안**).
- 제목은 `<span id="제목">제목</span>`으로 감싼다 — `[[#제목]]` 앵커용. 제목 안 서식은 살아 있다.
- 깊이를 건너뛰어도 번호는 다음 하위 번호를 쓴다: `== … ==` 다음 `==== … ====`는 `s-1.1`.
- 헤딩마다 그 뒤에 `<div class="wiki-heading-content">…</div>`가 열린다. **이 div는 중첩하지
  않는다** — 수준과 무관하게 헤딩마다 닫고 다시 연다 [렌더확정].

---

## 7. 리스트

**[도움말예제]** 줄머리에 **공백 하나 이상** + 마커. 공백이 없으면 리스트가 아니다.

```
 * 리스트 1        → <ul class="wiki-list"><li><div class="wiki-paragraph">리스트 1</div></li></ul>
* 리스트 1         → <div class="wiki-paragraph">* 리스트 1</div>   (앞 공백 없음)
```

### 7.1 마커와 클래스

| 마커 | 종류 | `<ol>`/`<ul>` 클래스 |
|---|---|---|
| `*` | 비순서 | `wiki-list` (`<ul>`) |
| `1.` | 십진 | `wiki-list wiki-list` (꼬리표 없어 두 번) |
| `a.` | 소문자 알파벳 | `wiki-list wiki-list-alpha` |
| `A.` | 대문자 알파벳 | `wiki-list wiki-list-upper-alpha` |
| `i.` | 소문자 로마 | `wiki-list wiki-list-roman` |
| `I.` | 대문자 로마 | `wiki-list wiki-list-upper-roman` |

- 순서 리스트는 `<ol … start="N">`. `start`는 첫 항목의 재지정 번호이고, 없으면 1이다.
- **`<li>`에는 속성이 없다** [렌더확정]. 모양은 `<ol>`의 클래스로, 시작 번호는 `<ol start>`로만
  나타낸다(`type="A"`·`<li value>`가 아니다).
- **시작 번호 재지정** [도움말예제]: `마커#N`. `I.#11 …` → `<ol … wiki-list-upper-roman" start="11">`.
- 마커 뒤 공백은 선택이다: `*붙은 내용`도 리스트다 [미확인].

### 7.2 중첩·여러 줄·빈 줄

- **중첩은 들여쓰기 깊이**로 표현한다 [도움말예제]. 더 깊이 들여쓴 마커 줄이 중첩 `<ul>`이
  된다. 종류가 섞여도 된다.
- **각 항목 내용은 `<div class="wiki-paragraph">`로 감싼다.**
- **빈 줄이 리스트를 끊는다** [도움말서술]. 끊긴 자리에는 빈 문단이 생긴다:
  `* 리스트 1` / (빈 줄) / `* 새 리스트` → 두 `<ul>` 사이에 `<div class="wiki-paragraph"></div>`.
- 리스트 안 개행은 `[br]` 매크로로 넣는다 [도움말예제].

빈 줄·들여쓰기의 임자(어느 항목·영역에 속하는가)는 들여쓰기 깊이로 갈린다. 세부 규칙은
복잡하며 구현의 `block.rs`가 다룬다(항목 속내용 뒤 빈 줄은 그 들여쓰기 안에 남는 등).

---

## 8. 인용문과 들여쓰기

### 8.1 인용문 — `>`

**[도움말예제]** 줄머리 `>`. 개수만큼 중첩한다.

```
>인용문         → <blockquote class="wiki-quote"><div class="wiki-paragraph">인용문</div></blockquote>
>인용문1 / >>인용문2 / >>>인용문3  → blockquote 3중 중첩
```

- **`>` 하나만이 마커다.** 뒤따르는 공백은 마커가 아니라 **들여쓰기 한 단계**다 [렌더확정]:
  - `>인용문` → `<blockquote><div class="wiki-paragraph">…`
  - `> 인용문` → `<blockquote><div class="wiki-indent"><div class="wiki-paragraph">…`
- `>` 뒤에 리스트·표를 바로 붙이면(`>* …`, 공백 없이) 마커로 인식되지 않는다. 공백을 두면
  인용문 안 리스트·표가 된다: `> * 목록`, `> || 표 ||` [도움말예제/미확인].
- 도움말은 최대 8단계라 하나, 렌더는 9단계도 9중 중첩으로 낸다 [도움말서술].

### 8.2 들여쓰기

**[도움말서술]** 줄머리 공백으로 들여쓴 문단은 `<div class="wiki-indent">`로 감싼다. 단계가
깊어지면 중첩한다.

```
 들여쓴 문단      → <div class="wiki-indent"><div class="wiki-paragraph">들여쓴 문단</div></div>
 한 단계 / (더 깊이) 두 단계  → wiki-indent 2중 중첩
```

---

## 9. 표

`|| 셀 || 셀 ||`. 셀은 `||`로 가른다. 행이 `||`로 닫히지 않으면 표가 아니라 문단이다.

```
|| 테이블 || 테이블 ||
→ <div class="wiki-table-wrap"><table class="wiki-table"><tbody>
    <tr class="wiki-table-tr">
      <td style="text-align: center;"><div class="wiki-paragraph">테이블</div></td>
      <td style="text-align: center;"><div class="wiki-paragraph">테이블</div></td>
    </tr></tbody></table></div>
```

구조 골격 [렌더확정]:

- 표는 `<div class="wiki-table-wrap">` > `<table class="wiki-table">` > `<tbody>`.
- **모든 `<tr>`에 `class="wiki-table-tr"`.**
- **모든 `<td>`는 내용이 무엇이든 `<div class="wiki-paragraph">`로 감싼다**(빈 셀은 빈 문단 하나).
  셀에 문단+리스트가 같이 있으면 래퍼를 더 씌우지 않고 나란히 둔다.
- **셀 안에서는 모든 개행이 `<br>`다** — 블록 요소(접기·표 등) 앞뒤에도 `<br>`가 남는다
  (`셀\n{{{#!folding …}}}\n끝` → `셀<br><details>…</details><br>끝`). 문서 최상단·일반 문단에서
  분류·`[include]` 줄의 개행이 사라지는 것과 다르다.

### 9.1 캡션

**[도움말예제]** 첫 줄 맨 앞에 `|캡션|`을 둔다.

```
|캡션| 테이블 || 내용 ||  → <table …><caption>캡션</caption>…
```

### 9.2 정렬

**[도움말서술/미확인]** 셀 옵션 `<(>`/`<:>`/`<)>` 또는 **셀 내용 좌우 공백**으로 정한다.

| 지정 | 결과 |
|---|---|
| `<(>` 또는 `||내용 ||`(뒤 공백만) | `text-align: left` |
| `<:>` 또는 `||내용 ||`(양쪽 공백) | `text-align: center` |
| `<)>` 또는 `|| 내용||`(앞 공백만) | `text-align: right` |
| 공백 없음 (`||내용||`) | 기본(왼쪽) — `text-align`을 방출하지 않음 |

공백 유도 정렬은 옵션이 없을 때만 적용된다. 옵션으로 명시하면 그 값이 이긴다.

### 9.3 병합

**[도움말예제]**

- **가로(colspan)**: `<-N>` 또는 파이프 런. `<-2>` → `colspan="2"`. `|||| 두칸 ||` → `colspan="2"`.
  `<-1>`처럼 1도 **적힌 대로** `colspan="1"`을 낸다 [미확인]. 파이프 쌍에서 온 colspan은
  2 이상일 때만 방출.
- **세로(rowspan)**: `<|N>`. `<-3><|2>` → `colspan="3" rowspan="2"`.
- **세로 정렬 겸용**: `<^|N>`(위), `<v|N>`(아래) → `rowspan="N"` + `vertical-align: top/bottom`.
- 위 행의 rowspan에 덮인 자리 때문에 셀이 없는 행도 `<tr class="wiki-table-tr"></tr>`로 빈다.

### 9.4 색상·스타일 옵션과 스코프

옵션 `<이름=값>`의 이름 접두사가 스코프를 정한다. 이름의 공백은 제거해 정규화한다
(`<table align=center>` = `<tablealign=center>`).

| 스코프 | 접두사 | 실리는 곳 | 옵션 예 |
|---|---|---|---|
| 셀 | (없음) | `<td style>` | `bgcolor`, `color`, `width`, `height`, `nopad`, `keepall` |
| 열 | `col` | 지정 셀부터 그 열 아래로 전파 | `colbgcolor`, `colcolor`, `coltextalign` |
| 행 | `row` | `<tr style>` | `rowbgcolor`, `rowcolor`, `rowtextalign` |
| 표 | `table` | wrap div 또는 `<table>` | `tablebgcolor`, `tablewidth`, `tablealign`, `tablebordercolor`, `tablecolor`, `tabletextalign` |

```
||<bgcolor=#ff0000> 빨강 ||     → <td style="text-align: center; background-color: #ff0000;">
||<rowbgcolor=#ff0000> 행 ||    → <tr … style=" background-color: #ff0000;">
||<colbgcolor=#ff0000> 열 ||    → 해당 열 아래 <td>에 background-color
||<tablebgcolor=#ff0000> 표 ||  → <table … style=" background-color: #ff0000;">
```

- **색상 값**: `#RRGGBB`·`#RGB`·CSS 색상명(`white`). `,`로 다크 쌍(다크 값은 `data-dark-style`).
  색이 아니면 그 선언을 통째로 버린다. `transparent`는 CSS 키워드라 받는다 [렌더확정].
- **우선순위** [도움말서술/렌더확정]: `bgcolor > colbgcolor > rowbgcolor > tablebgcolor`.
  셀과 열이 같은 속성을 주면 셀이 이긴다.
- **표 스코프의 두 갈래** [렌더확정]: 정렬은 wrap div 클래스(`table-center`/`-right`/`-left`),
  너비는 wrap div의 style(이때 `<table>`은 `width:100%`), 색·테두리는 `<table>`의 style.
  `tablebordercolor` → `border: 2px solid …`.

```
||<tablewidth=600px> 표 ||
→ <div class="wiki-table-wrap" style="width: 600px;"><table … style=" width: 100%;">…
||<tablealign=center> 표 ||  → <div class="wiki-table-wrap table-center">…
```

- 열 스코프 속성이 걸리는 **열 수는 옵션 순서**로 정해진다 [렌더확정]. 나무위키는 셀 옵션을
  왼쪽부터 처리해 그 자리까지 아는 칸 수만큼 건다: `<-3><colbgcolor=…>`는 세 열에,
  `<colbgcolor=…><-4>`는 적힌 시점에 한 칸뿐이라 한 열에만 건다.
- `<nopad>` → `<td class="wiki-table-nopadding">`.

**우리가 더 관대한 곳** [렌더확정]: 도움말이 미지원이라는 `<col bgcolor=>`(공백형)를 우리는
받는다(공백형은 `table` 계열만 허용이 정답). `<color#RRGGBB>`(등호 생략)는 도움말상 지원이나
우리는 옵션 파싱을 중단한다(→ 아래).

### 9.5 미지원 옵션·깨진 표

**[미확인]**

- **모르는 옵션에서 파싱이 중단**되고 그 자리부터 셀 본문 텍스트가 된다(정렬은 왼쪽):
  `||<우주=1> 셀 ||` → `<td style="text-align: left;"><div class="wiki-paragraph">&lt;우주=1&gt; 셀</div>`.
- 닫는 `||` 없는 행은 표가 아니라 문단: `|| 안 닫음` → `<div class="wiki-paragraph">|| 안 닫음</div>`.
- 행마다 셀 수가 달라도 그대로 낸다.
- **표는 빈 줄로 나뉜다** [도움말서술]. 붙은 두 표는 사이에 빈 줄이 있어야 별개 표다.

---

## 10. 각주

**[도움말예제]** `[* 내용]`. 이름은 `[*이름 내용]`. `[각주]` 매크로 자리에 각주 목록이 나온다.

```
[* 텍스트 1]
[각주]
→ <a class="wiki-fn-content" title="텍스트 1" href="#fn-1"><span id="rfn-1"></span>[1]</a>
  <br><div class="wiki-macro-footnote"><span class="footnote-list">
    <span id="fn-1"></span><a href="#rfn-1">[1]</a> 텍스트 1</span></div>
```

### 10.1 번호 모델

**[렌더확정]** 문서 안 **모든 각주 참조에 전역 일련번호**가 붙는다(재참조도 제 번호를 가진다).
**무명 각주의 라벨이 곧 그 번호**다.

- 이름 각주는 라벨이 이름이고, 번호는 `rfn-N`(역참조 앵커)에 쓰인다:
  `[*A 텍스트 2]` → `title="텍스트 2" href="#fn-A"` … `[A]`.
- 이름 각주가 번호를 차지하면 그다음 무명 각주는 그 번호를 건너뛴다 [미확인]:
  `[*A 가] [* 나] [*B 다] [* 라]` → 라벨 `[A] [2] [B] [4]`(무명은 2·4).

### 10.2 재참조

**[도움말예제]** 같은 이름을 여러 번 참조하면 목록에서 `<sup>첫번호.순번</sup>` 역참조가 여럿
달린다. 이름이 같으면 **맨 위 각주의 내용**으로 통일된다.

```
[*A 텍스트 2] [*A]
[각주]
→ … <span id="fn-A"></span>[A] <a href="#rfn-1"><sup>1.1</sup></a>
      <a href="#rfn-2"><sup>1.2</sup></a> 텍스트 2 …
```

### 10.3 참조·목록 구조

- 참조: `<a class="wiki-fn-content" title="<내용 글자>" href="#fn-{라벨}"><span id="rfn-{번호}"></span>[{라벨}]</a>`.
  `title`은 툴팁으로 보여줄 **각주 내용의 글자**다(리터럴·개행은 빠진다).
- 목록(한 번 참조): `<span id="fn-{라벨}"></span><a href="#rfn-{N}">[{라벨}]</a> 내용`.
- **각주 안의 각주는 금지** [도움말서술]. `[* 바깥 [* 안쪽]]`은 정상 표시되지 않는다(안쪽이
  먼저 닫혀 별개 각주가 된다).

---

## 11. 매크로

`[이름]` 또는 `[이름(인자)]`. 이름은 대소문자 무관. 모르는 매크로는 원문 표기 그대로 방출한다
(그래서 `[V]`, `[Chorus]` 같은 평범한 대괄호 텍스트도 화면상 그대로 남는다).

| 매크로 | 동작 | 등급 |
|---|---|---|
| `[목차]` / `[tableofcontents]` | 목차 | 도움말예제 |
| `[각주]` / `[footnote]` | 각주 목록 | 도움말서술 |
| `[br]` | 줄바꿈 `<br>` | 도움말서술 |
| `[clearfix]` | `<div class="wiki-clearfix">` | 도움말예제 |
| `[anchor(이름)]` | `<a id="이름">` | 도움말예제 |
| `[include(틀:…, 인자=값)]` | 틀 전개 (→ [11.2](#112-틀-호출--include)) | 도움말예제 |
| `[date]` / `[datetime]` | 현재 일시 | 도움말예제 |
| `[age(YYYY-MM-DD)]` | 만 나이 | 도움말서술 |
| `[dday(YYYY-MM-DD)]` | 디데이 `D±N` | 도움말서술 |
| `[youtube(id,width=,height=)]` | 유튜브 iframe (`wiki-media`) | 도움말예제 |
| `[kakaotv(id)]` / `[nicovideo(id)]` | 영상 iframe | 도움말서술/예제 |
| `[ruby(글자,ruby=…,color=…)]` | 루비 `<ruby>` | 도움말서술 |
| `[math(식)]` | 수식 `<span class="wiki-math">` | 미확인 |

```
[date]              → 2026-07-17 12:00:00
[age(2000-01-01)]   → 26
[dday(2000-01-01)]  → D+9694
[br]                → 앞<br>뒤 (앞[br]뒤)
[youtube(jNQXAC9IVRw)] → <iframe class="wiki-media" src="//www.youtube.com/embed/jNQXAC9IVRw" width="640" height="360" …>
[ruby(글자,ruby=루비,color=red)] → <ruby>글자<rp>(</rp><rt><span style="color:red">루비</span></rt><rp>)</rp></ruby>
```

**미구현·차이** [렌더확정]:

- `[pagecount]`, `[vimeo(…)]`, `[navertv(…)]` — the seed는 지원하나 우리는 미해결 매크로로
  원문을 노출한다.
- `[youtube]`의 `start=`/`end=` — the seed 지원, 우리는 폐기(크기만).
- `[목차(hide)]` — the seed는 접힌 목차, 우리는 인자를 무시하고 펼친다.

### 11.1 목차 구조

**[도움말예제/렌더확정]**

```
[목차]
== 문단 ==
→ <div class="wiki-macro-toc" id="toc"><details open><summary></summary>
    <div class="toc-indent"><span class="toc-item"><a href="#s-1">1</a>. 문단</span></div>
  </details></div>
```

- `<details open>`로 접을 수 있고, 깊이는 `<div class="toc-indent">` 중첩으로 표현한다.
- 항목은 `<span class="toc-item"><a href="#s-N">N</a>. 제목`. **번호 뒤 점은 링크 밖**이다
  (헤딩에서는 점이 링크 안인 것과 반대).
- **목차 제목은 헤딩 제목 그대로** — 링크·서식이 살아 있다. 단 `[anchor()]`는 뺀다(id 중복).

### 11.2 틀 호출 — `[include]`

**[도움말예제]** `[include(틀:이름, 인자1=값1, 인자2=값2)]`. 넘긴 인자로 틀 본문의 `@이름@`이
채워지고(→ [13.4](#134-틀-인자--이름)), `#!if` 조건식의 변수로도 쓰인다(→ [14장](#14-if-조건식-언어)).

```
[include(틀:다른 뜻 설명, 설명=이것은 설명입니다.)]   → 다른 뜻: 이것은 설명입니다.
```

- 값에 리터럴 쉼표를 넣으려면 `\,`로 이스케이프한다 [도움말서술].
- **틀 속의 틀은 확장하지 않는다** — 원문 노출 없이 조용히 버려진다 [렌더확정]:
  `[include(틀:바깥, 인자=[include(틀:안쪽)])]` → `바깥`. 이 규칙이 순환을 구조적으로 막는다.
- **줄 첫머리의 `[include]`는 제 문단**, 줄 중간이면 현재 문단에 중첩된다 [렌더확정].
- 틀 안에서 쓴 같은 문서 앵커에는 `i{N}-` 접두사가 붙는다(`N`은 문서 안 include 순번).
  다른 문서의 앵커는 그대로다.

---

## 12. 중괄호 그룹

`{{{ … }}}`. 첫 토큰(`#!wiki` 등)이 종류를 정한다. **모두 인라인 요소**이며 안에 블록을
품을 수 있다([1.1](#11-블록과-인라인)).

닫히지 않은 `{{{`는 그룹이 아니라 글자다. the seed는 `{` 하나를 글자로 흘리고 다음 자리에서
다시 연다(`{{{{{{-5 …`의 복구). 그룹 사이에 `{`나 `}`를 3개 이상 연속으로 쓸 수 없다.

### 12.1 리터럴 — `{{{ … }}}`

**[도움말예제/미확인]** 지시자 없는 그룹은 안쪽 문법을 적용하지 않는다.

```
{{{[[리터럴]]}}}   → <code>[[리터럴]]</code>         (한 줄 → <code>)
{{{
코드 '''그대로'''
}}}                → <pre><code>코드 '''그대로'''</code></pre>   (여러 줄 → <pre><code>)
{{{{{{}}}}}}       → <code>{{{}}}</code>
```

한 줄 리터럴은 `<code>`, 여러 줄은 `<pre><code>`.

### 12.2 `#!wiki` — 스타일 컨테이너

**[도움말예제/서술]** `style=`·`dark-style=` 속성을 실은 `<div>`. 속성 뒤에 줄바꿈을 두고
내용을 이어 쓴다(한 줄 형태 `{{{#!wiki style="…" 내용}}}`도 된다).

```
{{{#!wiki style="text-align:center"
가운데
}}}                → <div style="text-align: center">가운데</div>
```

- 홑따옴표·겹따옴표 모두 가능. `style`을 두 번 주면 이어 붙는다.
- `dark-style`은 `data-dark-style`로 나간다.
- **`#!wiki` div에는 클래스가 없다**(`<div style=…>`뿐, `wiki-style` 클래스는 the seed에 없다) [렌더확정].
- 문단 래퍼를 두지 않는 컨테이너다 — 안쪽 내용이 바로 온다. 안에 표·리스트 등 블록을 품는다.
- **style 필터**: 나무위키는 `style` 값을 화이트리스트로 거른다(→ [부록 B](#부록-b-wiki-style-필터)).

### 12.3 `#!folding` — 접기

**[도움말예제]** `<details class="wiki-folding">`. 첫 줄 나머지가 접기 문구(summary)다.

```
{{{#!folding [ 펼치기 · 접기 ]
내용
}}}
→ <details class="wiki-folding"><summary>[ 펼치기 · 접기 ]</summary><div>내용</div></details>
```

- 문구가 없으면 기본값 `More` [도움말예제].
- **접기 문구에는 위키 문법이 적용되지 않는다** — 글자 그대로다(틀 인자만 값이 된다) [렌더확정].
- 내용 div도 컨테이너다(문단 래퍼 없음).

### 12.4 `#!syntax` — 구문 강조

**[도움말서술]** `{{{#!syntax 언어\n코드}}}`.

```
{{{#!syntax python
print(1)
}}}                → <pre><code class="hljs" data-language="python">print(1)</code></pre>
```

클래스·`data-language`는 the seed와 맞추나, **구문 강조 토큰화(highlight.js)는 미구현**이다 —
코드는 통짜 텍스트로 나간다 [렌더확정, 설계 판단].

### 12.5 `#!html` — 원시 HTML

**[도움말서술]** 나무위키는 `#!html`을 원시 HTML로 렌더한다. 위키 입력이 원시 HTML로 나가는
유일한 통로라 **화이트리스트 sanitizer**를 거친다(→ [부록 C](#부록-c-html-sanitizer)).

```
{{{#!html <span style="background-color: #999">서술할 내용</span>}}}
→ <span style="background-color: #999">서술할 내용</span>
{{{#!html <script>alert(1)</script>}}}   → (빈 문단 — script는 내용까지 폐기)
{{{#!html <marquee>글자</marquee>}}}     → 글자 (미허용 태그는 껍데기만 벗김)
{{{#!html a&nbsp;b}}}                     → a b (엔티티는 글자로 디코드)
```

### 12.6 `#!if` — 조건식

**[미확인, the seed 지원]** 도움말엔 없으나 `틀:상위 문서` 등이 쓴다. 조건이 참일 때만 내용을
렌더하며, 조건식이 **값을 내면서 변수도 만드는** 작은 표현식 언어다. 인자를 치환하기 **전에**
`#!if`를 먼저 해결한다.

```
{{{#!if 1
보임}}}             → 보임
{{{#!if null
숨김}}}             → (없음)
```

`[include]`로 넘어온 인자와 조합된다:

```
[include(틀:상위 문서, 문서명1=문서)]
→ 상위 문서: <a class="wiki-link-internal" href="/w/%EB%AC%B8%EC%84%9C" title="문서">문서</a>
```

조건식 문법은 [14장](#14-if-조건식-언어).

### 12.7 색상·크기 그룹 안의 블록

**[미확인]** 색상·크기 그룹은 서식이라 안쪽을 인라인으로 편다. 표처럼 인라인으로 펼 수 없는
블록을 감싸면 버려진다.

```
{{{#red
|| 표 ||
}}}                → <span style="color:red" …></span>   (표가 버려짐)
```

`#!if`는 이와 달리 블록을 그대로 보존한다(감싸는 요소를 만들지 않으므로):

```
{{{#!if 1
|| 표 ||
}}}                → <div class="wiki-table-wrap">…표…</div>
```

---

## 13. 주석·리다이렉트·분류·틀 인자

### 13.1 주석

**[도움말예제]** 줄머리 `##`. 개행까지 통째로 사라진다(문단도 표도 끊지 않는다).

```
## 주석
본문                → <div class="wiki-paragraph">본문</div>
```

- **앞에 공백이 있으면 주석이 아니다** [도움말서술]: ` ## 주석 아님` → 들여쓴 문단으로 그대로 출력.
- `##@`는 편집 창 전용 고정 주석 — 읽기 모드에선 안 나온다 [도움말예제].
- `{{{ }}}` 그룹 **안의 `##`은 주석이 아니라 글자**다(리터럴 예제 보존).

### 13.2 리다이렉트

**[도움말예제]** `#redirect 목적지` 또는 `#넘겨주기 목적지`. 문서 첫 줄에 그것만 있어야 한다.
본문 렌더는 비어 있다(리다이렉트 메타).

```
#redirect 목적지 문서       → (본문 없음)
#redirect 목적지 문서#s-1   → (문단으로 리다이렉트)
```

첫 줄이 아니면 리다이렉트가 아니라 글자다 [도움말예제].

### 13.3 분류

**[도움말서술/렌더확정]** `[[분류:이름]]`. 본문이 아니라 스킨이 그리는 분류 바(`wiki-categories`)로
나간다. **분류만 있는 줄은 개행까지 사라진다.**

```
[[분류:알파위키]]   → <div class="wiki-categories">분류: <a href="/w/…%3A알파위키…">알파위키</a></div>
```

- `[[:분류:…]]`(콜론 접두사)는 분류 등록이 아니라 텍스트 링크 → [4.3](#43-상대-링크--이름공간).
- `[[분류:X|정렬키]]`의 정렬키는 색인용이라 표시에 영향 없음(우리는 폐기) [도움말서술].
- **`[[분류:X#blur]]`의 `#blur`가 분류명을 오염**시키는 버그가 있다(the seed는 href에서 분리) [렌더확정].

### 13.4 틀 인자 — `@이름@`

**[도움말서술]** `@이름@` 또는 `@이름=기본값@`. 나무마크 구조를 만들지 않으므로(도움말: 나무마크
자체 문법엔 매개변수 사용 불가) 값이 정해지기 전에도 구조는 확정된다. `[include]`가 실어 온
인자로 값이 정해지고, 없으면 기본값, 그것도 없으면 빈 문자열이다.

```
@매개변수@           → (빈 문단)
@매개변수=디폴트값@  → 디폴트값
```

이름·기본값에는 `@`와 줄바꿈이 올 수 없다. 이 규칙 덕에 본문의 평범한 `@`(이메일 등)는
인자로 오인되지 않는다.

---

## 14. `#!if` 조건식 언어

**[미확인, the seed 실사용에서 관찰]** `#!if`의 조건식은 나무마크가 아니라 틀(include) 인자를
다루는 별도의 표현식 언어다. 대입으로 변수를 만들고, 내용의 `@이름@`이 그 변수를 참조한다.

```
{{{#!if top = 문서명1 != null ? 문서명1 : calleeTitle
상위 문서: [[@top@]]}}}
```

문법 범위는 나무위키 틀에서 실제로 관찰된 만큼이다.

- 시퀀스(`,`), 대입(`=`), 삼항(`? :`)
- 논리: `||`(OR), **`&`**(AND — 틀은 논리 AND에 단일 앰퍼샌드를 쓴다)
- 비교: `==`, `!=`
- 연산: `+`(문자열 이음)
- 속성·메서드: `.length`, `startsWith(…)`, `substr(…)`, `lastIndexOf(…)`, `?.`(옵셔널 체이닝)
- 괄호, `null`
- `calleeTitle` — 호출자 문서명(`WikiContext::current_title()`이 공급)

평가는 참/거짓과 변수 바인딩을 함께 돌려준다. 변수는 뒤따르는 `#!if`로 누적된다
(`틀:하위 문서`가 앞에서 만든 변수를 뒤에서 재사용). 구현은
[`crates/namumark-render/src/condition.rs`](../../crates/namumark-render/src/condition.rs).

**중첩 include는 확장하지 않는다** [렌더확정]. 틀 속의 틀(`[include(틀:X)]` 안의 `[include(…)]`)은
원문 노출 없이 조용히 버려진다. 이 규칙이 순환을 구조적으로 막는다.

---

## 부록 A. CSS 색상명 148개

`{{{#이름}}}`과 표 `<bgcolor=이름>`에서 색으로 받는 이름. 정식 CSS 색상 이름만이다
(`transparent`는 여기 없지만 표 스코프에선 CSS 키워드로 받는다).

```
aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue
blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk
crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki
darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen
darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue
dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite
gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki
lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan
lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen
lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen
magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen
mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream
mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid
palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum
powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown
seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen
steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen
```

## 부록 B. `#!wiki` style 필터

나무위키는 `#!wiki`의 `style` 값을 화이트리스트로 거른다. **증거가 있는 것만 막는다**
(목록을 넘겨짚으면 멀쩡한 CSS가 조용히 사라진다).

- **`image-rendering`** — 속성째 폐기(도움말은 동작하는 것처럼 서술하나 the seed 렌더에 없다).
- **`display`** — CSS 키워드 값만 통과(`display: 5ine`처럼 무효 값은 그 선언째 폐기).
  통과 키워드: `block contents flex flow-root grid inline inline-block inline-flex inline-grid
  inline-table list-item none table table-caption table-cell table-column table-column-group
  table-footer-group table-header-group table-row table-row-group`.
- **함수 안 함수**가 든 값은 그 선언째 폐기: `repeating-linear-gradient(45deg, #1f719a 6%, …)`는
  받지만 `linear-gradient(0deg, rgba(255,255,255,.875), …)`는 안 받는다.

값이 무효이거나 위 규칙에 걸린 선언은 버리고, 남는 선언이 없으면 `style` 속성 자체를 두지
않는다. `data-dark-style`도 같은 필터를 지난다.

## 부록 C. `#!html` sanitizer

`#!html` 원문을 안전한 부분집합으로 거른다.

- **허용 태그(17)**: `a b i u s strong em sub sup br span div code small big wbr video`.
- **허용 속성(7)**: `class href style src width height controls`.
  - `id`는 문서 앵커(`s-1`, `fn-1`)와 충돌해 받지 않는다.
  - `href`는 나무마크 외부 링크와 같은 수준으로 깎는다(http/https/ftp만, `target=_blank
    rel="nofollow noopener ugc"`를 우리가 붙임).
  - `src`·`width`·`height`·`controls`는 `<video>` 전용. 이미지의 `src`는 나무마크 문법이 대신한다.
- **style**: `url(`·`expression(`·`javascript:`·`@import`·`behavior:`가 있으면 통째로 폐기.
- **미허용 태그**: 껍데기만 벗기고 글자는 남긴다. 단 `script`·`style`·`iframe`·`object`·`embed`는
  내용까지 폐기.
- **미닫힘 태그**는 sanitizer가 닫는다.
- **엔티티는 글자로 디코드**(`&nbsp;`→U+00A0, `&#8203`→U+200B, `&commat;`→`@`; 세미콜론 없어도
  받음). 디코드한 글자가 `<`·`>`·`&`면 재이스케이프해 우회를 막는다.

## 부록 D. 퍼센트 인코딩

| 자리 | hex | 특수문자 |
|---|---|---|
| 문서 경로(`/w/…`) | 대문자 | `:`·`/`·`(`·`)` 미인코딩, 공백 `%20` |
| 각주 앵커(`#fn-…`) | 소문자 | 전부 인코딩 |
| 분류 푸터 링크 | 대문자 | `:`을 `%3A`로 인코딩 |

## 부록 E. 주요 클래스 어휘

the seed와 같은 클래스를 쓴다. 대조·구현의 앵커다.

`wiki-paragraph` · `wiki-heading` · `wiki-heading-folded` · `wiki-heading-content` ·
`wiki-list`(+`-alpha`/`-upper-alpha`/`-roman`/`-upper-roman`) · `wiki-quote` · `wiki-indent` ·
`wiki-table-wrap`(+`table-center`/`-right`/`-left`) · `wiki-table` · `wiki-table-tr` ·
`wiki-table-nopadding` · `wiki-link-internal`(+`not-exist`) · `wiki-self-link` ·
`wiki-link-external` · `wiki-image-align-normal`/`-left`/`-center`/`-right` ·
`wiki-image-wrapper` · `wiki-image-theme-dark` · `wiki-macro-toc` · `toc-indent` · `toc-item` ·
`wiki-macro-footnote` · `footnote-list` · `wiki-fn-content` · `wiki-folding` · `wiki-clearfix` ·
`wiki-media` · `wiki-math` · `wiki-categories`.

색상은 `style="color:#fff"` + `data-dark-style="color:#fff;"`(다크 미지정도 같은 값)로 낸다.

---

## 근거·재현

이 문서의 규칙은 [`fixtures/corpus/`](../../fixtures/corpus)의 287개 케이스가 검증한다. 케이스는
`원문 → 의미 모델 → 렌더 HTML`을 자기완결적으로 담고 근거 등급을 함께 적는다. 대조 방법론과
도구는 [`tools/parity/README.md`](../../tools/parity/README.md)에 있다.

> 문서 본문 예제 일부는 알파위키 문법 도움말([CC BY-NC-SA 2.0 KR](https://creativecommons.org/licenses/by-nc-sa/2.0/kr/))에서
> 옮긴 것이다. 저장소 코드의 MIT 라이선스와 분리된다.
