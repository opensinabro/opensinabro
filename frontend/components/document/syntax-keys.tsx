"use client";

import type { RefObject } from "react";

// 모바일에서 편집을 가장 크게 막는 것은 좁은 화면이 아니라 자판이다. `'''`·`[[ ]]`·`||`는
// 기본 자판에서 기호 판을 두 번 오가야 나오고, 그 사이에 쓰던 자리를 놓친다. 그래서
// 자판 바로 위에 나무마크 기호만 모은 줄을 붙인다 — 데스크탑에는 물리 자판이 있으므로
// 세우지 않는다.
type SyntaxKey = {
  label: string;
  /** 고른 글을 감싸는 것(굵게·링크). */
  wrap?: [string, string];
  /** 커서가 놓인 줄 전체를 감싸는 것(문단·목록). 줄 단위 문법이라 자리가 줄머리다. */
  line?: [string, string];
};

const keys: SyntaxKey[] = [
  { label: "'''굵게'''", wrap: ["'''", "'''"] },
  { label: "''기울''", wrap: ["''", "''"] },
  { label: "__밑줄__", wrap: ["__", "__"] },
  { label: "~~취소~~", wrap: ["~~", "~~"] },
  { label: "[[링크]]", wrap: ["[[", "]]"] },
  { label: "== 문단 ==", line: ["== ", " =="] },
  { label: "* 목록", line: [" * ", ""] },
  { label: "|| 표 ||", wrap: ["||", "||"] },
  { label: "[* 각주]", wrap: ["[* ", "]"] },
];

function lineBounds(value: string, from: number, to: number) {
  const head = value.lastIndexOf("\n", from - 1) + 1;
  const tail = value.indexOf("\n", to);

  return [head, tail === -1 ? value.length : tail] as const;
}

export function SyntaxKeys({
  textarea,
}: {
  textarea: RefObject<HTMLTextAreaElement | null>;
}) {
  function apply(key: SyntaxKey) {
    const element = textarea.current;
    if (element === null) return;

    const [open, close] = (key.wrap ?? key.line) as [string, string];
    const [from, to] = key.line
      ? lineBounds(element.value, element.selectionStart, element.selectionEnd)
      : [element.selectionStart, element.selectionEnd];

    const inner = element.value.slice(from, to);

    // 값을 직접 갈아 끼우지 않고 브라우저의 입력 명령으로 넣는다. 상태로 덮어쓰면
    // 실행 취소 기록이 통째로 끊겨, 문법 단추를 한 번 누른 뒤로는 손으로 친 글까지
    // 되돌릴 수 없게 된다. 이 명령은 사람이 친 것과 같은 입력으로 취급되어 기록이
    // 이어지고, 리액트도 그 input 사건을 받아 상태를 따라온다.
    element.focus();
    element.setSelectionRange(from, to);
    globalThis.document.execCommand("insertText", false, open + inner + close);

    // 감싼 글은 고른 채로 둔다. 선택이 풀리면 두 겹을 연달아 씌울 수 없고(굵은 링크),
    // 무엇이 바뀌었는지도 눈으로 좇기 어렵다. 값은 이미 DOM에 들어가 있으므로 여기서
    // 바로 잡아도 다음 렌더가 흔들지 않는다.
    element.setSelectionRange(from + open.length, from + open.length + inner.length);
  }

  return (
    <div className="flex flex-none gap-1.5 overflow-x-auto border-t border-line bg-ground-sub px-2.5 py-1.5 [scrollbar-width:none] lg:hidden [&::-webkit-scrollbar]:hidden">
      {keys.map((key) => (
        <button
          key={key.label}
          type="button"
          // 누르는 순간 원문이 초점을 잃으면 커서 자리가 사라져 어디를 감쌀지 알 수
          // 없어진다. 초점은 원문에 그대로 두고 누름만 받는다.
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => apply(key)}
          className="text-fine h-9 flex-none rounded border border-line bg-ground px-2.5 font-mono whitespace-nowrap text-accent-deep active:border-accent active:bg-accent-wash"
        >
          {key.label}
        </button>
      ))}
    </div>
  );
}
