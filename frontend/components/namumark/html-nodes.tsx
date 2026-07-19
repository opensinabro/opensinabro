import { createElement, type ReactNode } from "react";
import type { HtmlNode } from "@/lib/namumark/HtmlNode";
import type { HtmlTag } from "@/lib/namumark/HtmlTag";
import { styleObject } from "./style";

/**
 * `#!html`이 실어 온 요소를 그린다.
 *
 * `dangerouslySetInnerHTML`을 쓰지 않는다 — IR이 화이트리스트를 통과한 노드만 담고
 * 있으므로 여기서 다시 정제할 것이 없고, 정제할 것이 없으니 문자열로 주입할 이유도 없다.
 */
const tagName: Record<HtmlTag, string> = {
  anchor: "a",
  bold: "b",
  italic: "i",
  underline: "u",
  strikethrough: "s",
  strong: "strong",
  emphasis: "em",
  subscript: "sub",
  superscript: "sup",
  lineBreak: "br",
  span: "span",
  division: "div",
  code: "code",
  small: "small",
  big: "big",
  wordBreakOpportunity: "wbr",
  video: "video",
};

const voidTags: ReadonlySet<HtmlTag> = new Set<HtmlTag>([
  "lineBreak",
  "wordBreakOpportunity",
]);

export function HtmlNodes({ nodes }: { nodes: HtmlNode[] }): ReactNode {
  return nodes.map((node, index) => <HtmlNodeView key={index} node={node} />);
}

function HtmlNodeView({ node }: { node: HtmlNode }): ReactNode {
  if (node.type === "text") {
    return node.text;
  }

  const { tag, attributes } = node;
  const isLink = tag === "anchor";
  const properties: Record<string, unknown> = {
    // 링크 차림새는 우리가 정한다 — 위키 입력이 UI 클래스를 사칭하지 못한다.
    className: isLink ? "wiki-link-external" : (attributes.class ?? undefined),
    style: styleObject(attributes.style),
  };
  if (attributes.href !== null) {
    properties.href = attributes.href;
  }
  if (isLink) {
    properties.target = "_blank";
    properties.rel = "nofollow noopener ugc";
  }
  if (tag === "video") {
    if (attributes.source !== null) properties.src = attributes.source;
    if (attributes.width !== null) properties.width = attributes.width;
    if (attributes.height !== null) properties.height = attributes.height;
    if (attributes.controls) properties.controls = true;
  }

  if (voidTags.has(tag)) {
    return createElement(tagName[tag], properties);
  }
  return createElement(
    tagName[tag],
    properties,
    <HtmlNodes nodes={node.children} />,
  );
}
