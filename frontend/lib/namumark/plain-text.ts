import type { RenderInline } from "./RenderInline";

// 목차 항목의 제목은 인라인 트리다 — 링크·강조가 살아 있다. 우측 축은 그 제목을
// 접근 이름과 눈금 길이에만 쓰므로 글자만 필요하다. 트리를 그대로 넘기면 축이
// 인라인 렌더러 전체를 클라이언트 번들로 끌고 들어온다.
export function plainText(inlines: RenderInline[]): string {
  return inlines.map(one).join("");
}

function one(inline: RenderInline): string {
  switch (inline.type) {
    case "text":
    case "literal":
      return inline.text;
    case "styled":
    case "colored":
    case "sized":
      return plainText(inline.content);
    case "documentLink":
      return plainText(inline.display);
    case "externalLink":
      return inline.display ? plainText(inline.display) : "";
    case "ruby":
      return inline.content;
    case "math":
      return inline.formula;
    case "lineBreak":
      return " ";
    default:
      // 각주 참조·이미지처럼 제목의 글자가 아닌 것들. 헤딩에 와도 이름에 보태지 않는다.
      return "";
  }
}
