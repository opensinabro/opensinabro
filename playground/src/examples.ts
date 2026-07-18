export interface Example {
  id: string
  label: string
  source: string
}

export const EXAMPLES: Example[] = [
  {
    id: 'intro',
    label: '소개',
    source: `= 나무마크 플레이그라운드 =
왼쪽에 '''나무마크'''를 입력하면 오른쪽에 실시간으로 렌더됩니다.

== 문법 맛보기 ==
 * 리스트 항목
 * ''기울임''과 __밑줄__, ~~취소선~~
 * {{{#!wiki style="color:red"
   색이 있는 상자}}}

[[문서 링크]]와 [[https://namu.wiki|바깥 링크]], 각주도 됩니다.[* 이렇게요.]

|| 표 || 헤더 ||
|| 셀 || 셀 ||
`,
  },
  {
    id: 'text',
    label: '텍스트 서식',
    source: `= 텍스트 서식 =
'''굵게''', ''기울임'', __밑줄__, ~~물결 취소선~~, --하이픈 취소선--

윗첨자 X^^2^^ 와 아래첨자 H,,2,,O.

색: {{{#ff0000 빨강}}}, {{{#0275d8 파랑}}}, {{{#2e8b57 초록}}}.

크기: {{{+2 크게}}}, 보통, {{{-2 작게}}}.
`,
  },
  {
    id: 'heading',
    label: '문단·목차',
    source: `[목차]

= 개요 =
가장 큰 문단이다.

== 배경 ==
=== 세부 ===
[[#개요|개요로 이동]]하거나 [anchor(표식)] 자리를 만들 수 있다.

== 정리 ==
문단마다 자동으로 번호와 목차 항목이 붙는다.
`,
  },
  {
    id: 'list',
    label: '리스트·인용',
    source: `= 리스트 =
 * 순서 없는 항목
 * 항목
  * 한 단계 중첩
 1. 순서 있는 항목
 1. 항목

= 인용 =
>인용문입니다.
>>중첩 인용도 됩니다.

= 들여쓰기 =
 한 칸 들여쓴 줄.
  두 칸 들여쓴 줄.
`,
  },
  {
    id: 'table',
    label: '표',
    source: `= 표 =
|손익 계산| 항목 || 금액 ||
||<width=120px> 매출 ||<:> 1,000 ||
||<-2><bgcolor=#e0ffe0> 두 칸을 병합한 셀 ||
||<|2> 두 줄 병합 || 위 칸 ||
|| 아래 칸 ||
`,
  },
  {
    id: 'macro',
    label: '매크로·코드',
    source: `= 매크로 =
줄바꿈: 앞[br]뒤

루비: [ruby(漢字,ruby=한자)]

수식: [math(E=mc^2)]

= 코드 블록 =
{{{#!syntax rust
fn main() {
    println!("안녕, 나무마크!");
}
}}}

= 접기 =
{{{#!folding [ 펼치기 · 접기 ]
접힌 내용입니다.
}}}
`,
  },
  {
    id: 'link',
    label: '링크·앵커',
    source: `= 문서 링크 =
안쪽 문서: [[문서]], 다른 이름으로 [[문서|이렇게 보이게]]

하위·상위 문서: [[/심화]], [[../]]

= 바깥 링크 =
이름 붙여: [[https://www.google.com/|구글]]

그냥 주소만: [[https://namu.wiki]]

= 문단·앵커 =
문단으로: [[#s-1|첫 문단으로]], 표식으로: [[#표식|표식 자리]]

[anchor(표식)] 여기가 표식이 놓인 자리다.

= 리터럴 =
문법을 그대로 보여주려면: {{{[[문서]] '''굵게''' [* 각주]}}}
`,
  },
  {
    id: 'list-advanced',
    label: '리스트 심화',
    source: `= 번호 종류 =
 1. 십진수
 1. 다음 항목
 a. 소문자 알파벳
 a. 다음
 A. 대문자 알파벳
 i. 소문자 로마자
 I. 대문자 로마자

= 시작 번호 지정 =
 I.#11 로마자 11(XI)부터
 I. 다음은 12(XII)

= 깊은 중첩 =
 * 1단계
  * 2단계
   * 3단계
    1. 번호와 불릿 섞기
    1. 다음
`,
  },
  {
    id: 'code',
    label: '코드 하이라이트',
    source: `= 여러 언어 =
{{{#!syntax rust
fn fib(n: u64) -> u64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}
}}}

{{{#!syntax python
def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)
}}}

{{{#!syntax javascript
const fib = (n) => (n < 2 ? n : fib(n - 1) + fib(n - 2))
}}}

= 강조 없는 코드 =
{{{
언어 지정 없이 원문 그대로.
    들여쓰기와 [[문법]]이 보존된다.
}}}
`,
  },
  {
    id: 'article',
    label: '종합 문서',
    source: `[목차]

= 개요 =
'''나무마크'''는 나무위키에서 쓰이는 경량 마크업이다.[* 이 문서는 예시입니다.]

|| '''항목''' || '''내용''' ||
|| 종류 || 경량 마크업 ||
|| 용도 ||<bgcolor=#eef> 위키 문서 작성 ||

== 특징 ==
 * 사람이 읽고 쓰기 쉽다
 * ''기울임'', '''굵게''', __밑줄__ 같은 서식을 지원한다
 * 표와 [[#각주|각주]], 링크를 문장 안에 자연스럽게 넣는다

== 예시 ==
> 인용문으로 강조할 수도 있다.

{{{#!wiki style="background-color:#f6f8fa; padding:10px; border-radius:6px"
상자로 묶어 눈에 띄게 만들 수도 있다.
}}}

== 각주 ==
각주는 문서 아래에 모여 번호로 이어진다.[* 이렇게 두 번째 각주가 붙는다.]
`,
  },
  {
    id: 'box',
    label: '상자·레이아웃',
    source: `= wiki 상자 =
{{{#!wiki style="border:1px solid #ccc; padding:12px; border-radius:6px"
테두리와 안쪽 여백을 가진 상자.
}}}

{{{#!wiki style="background-color:#e8f4ff; text-align:center; padding:12px"
가운데 정렬된 파란 배경 상자.
}}}

= 다크 모드 대응 =
{{{#!wiki style="background-color:#fff; color:#111" dark-style="background-color:#2d2f34; color:#eee"
라이트/다크 각각의 색을 지정할 수 있다.
}}}

= 접기 =
{{{#!folding 접힌 리스트 펼치기
 * 첫째 항목
 * 둘째 항목
}}}
`,
  },
  {
    id: 'media',
    label: '미디어 임베드',
    source: `= 유튜브 =
[youtube(jNQXAC9IVRw)]

크기와 재생 구간:
[youtube(jNQXAC9IVRw,width=480,height=270,start=30)]

= 다국어 코드 하이라이트 =
{{{#!syntax python
def greet(name):
    print(f"안녕, {name}!")
}}}
`,
  },
  {
    id: 'html',
    label: 'HTML·조건·틀',
    source: `= 원시 HTML =
{{{#!html
<span style="background-color:#ffe08a; padding:2px 6px; border-radius:4px">강조 배지</span>
}}}

= 조건부 서술 =
조건이 참이면 안쪽 내용을 보여준다.
{{{#!if 1
이 문장은 조건이 참이라 나타난다.}}}
{{{#!if 0
이 문장은 조건이 거짓이라 숨겨진다.}}}

= 틀 변수 =
기본값을 가진 변수는 단독 렌더에서도 값을 채운다: @지역=서울@, @언어=한국어@.
`,
  },
  {
    id: 'misc',
    label: '기타 서식',
    source: `= 수평줄 =
위 문단
----
아래 문단

= 분류 =
[[분류:예시 문서]]
[[분류:플레이그라운드]]

= 각주 =
본문에 붙는 각주[* 일반 각주]와 이름 있는 각주[*주1 이름 붙은 각주]를 쓴다.
같은 이름을 다시 참조[*주1]하면 같은 번호가 매겨진다.

= 주석 =
보이는 줄
##이 줄은 주석이라 렌더되지 않습니다
다시 보이는 줄

= 이스케이프 =
문법 기호를 그대로: \\'\\'기울임 아님\\'\\', \\[대괄호 그대로\\]
`,
  },
]
