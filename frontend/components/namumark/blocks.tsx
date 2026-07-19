import { Fragment, createElement, type ReactNode } from "react";
import { Link } from "@/components/layout/link";
import type { ListKind } from "@/lib/namumark/ListKind";
import type { RenderBlock } from "@/lib/namumark/RenderBlock";
import type { RenderContext } from "./context";
import { Inlines } from "./inlines";
import { Table } from "./table";
import { classNames } from "./style";

export function Blocks({
  blocks,
  context,
}: {
  blocks: RenderBlock[];
  context: RenderContext;
}): ReactNode {
  return blocks.map((block, index) => (
    <Block key={index} block={block} context={context} />
  ));
}

/**
 * 문단 래퍼를 두지 않는 컨테이너(`#!wiki`·접기) 안의 블록들.
 *
 * 앞 블록에서 이어지는 문단 앞에 줄바꿈 하나를 둔다 — 나무위키가 그렇게 한다.
 */
export function ContainerBlocks({
  blocks,
  context,
}: {
  blocks: RenderBlock[];
  context: RenderContext;
}): ReactNode {
  return blocks.map((block, index) =>
    block.type === "paragraph" ? (
      <Fragment key={index}>
        {index > 0 && <br />}
        <Inlines inlines={block.content} context={context} />
      </Fragment>
    ) : (
      <Block key={index} block={block} context={context} />
    ),
  );
}

export type HeadingBlock = Extract<RenderBlock, { type: "heading" }>;

/**
 * 절 제목 줄의 속.
 *
 * 여닫이 셰브런 · 절 번호 · 제목 · 편집 순으로 선다. 절의 주소는 번호가 겸하므로
 * 앵커라는 표시를 따로 두지 않는다. 여닫이로 감싸는 자리(render-tree)와 그럴 수
 * 없는 자리(표 칸 안)가 같은 줄을 쓰도록 속만 떼어 둔다.
 */
export function HeadingLine({
  block,
  foldable,
  context,
}: {
  block: HeadingBlock;
  /** 여닫이로 감싸인 자리인가. 접을 수 없는 자리에는 셰브런을 두지 않는다. */
  foldable: boolean;
  context: RenderContext;
}): ReactNode {
  return (
    <>
      {foldable && <span className="wiki-chevron" aria-hidden="true" />}
      {createElement(
        `h${Math.min(Math.max(block.level, 1), 6)}`,
        { className: "wiki-heading-title" },
        <a className="wiki-heading-number" href={`#s-${block.number}`}>
          {block.number}.
        </a>,
        " ",
        // 제목 글자로 건 문단명 앵커. `[[#개요]]`가 이걸 가리킨다.
        <span id={block.anchor}>
          <Inlines inlines={block.content} context={context} />
        </span>,
      )}
      {context.editPath !== null && (
        <Link className="wiki-heading-edit" href={context.editPath}>
          편집
        </Link>
      )}
    </>
  );
}

const listClass: Record<ListKind, string> = {
  unordered: "wiki-list",
  // 십진 리스트는 꼬리표가 없어 클래스가 두 번 나온다 — 나무위키 표기 그대로다.
  decimal: "wiki-list wiki-list",
  lowerAlphabet: "wiki-list wiki-list-alpha",
  upperAlphabet: "wiki-list wiki-list-upper-alpha",
  lowerRoman: "wiki-list wiki-list-roman",
  upperRoman: "wiki-list wiki-list-upper-roman",
};

function Block({
  block,
  context,
}: {
  block: RenderBlock;
  context: RenderContext;
}): ReactNode {
  switch (block.type) {
    // 문서 최상위 절은 render-tree가 여닫이로 감싸 그린다. 여기로 오는 것은
    // 그럴 수 없는 자리(표 칸·접기 안 등)의 헤딩이라 여닫이 없이 줄만 세운다.
    case "heading":
      return (
        <div
          className={classNames(
            "wiki-heading",
            block.folded && "wiki-heading-folded",
          )}
        >
          <HeadingLine block={block} foldable={false} context={context} />
        </div>
      );

    // 문서 끝 각주는 문단으로 감싸지 않고 맨 블록으로 온다.
    case "paragraph":
      if (
        block.content.length === 1 &&
        block.content[0].type === "footnoteSection"
      ) {
        return <Inlines inlines={block.content} context={context} />;
      }
      return (
        <div className="wiki-paragraph">
          <Inlines inlines={block.content} context={context} />
        </div>
      );

    case "horizontalRule":
      return <hr />;

    case "quote":
      return (
        <blockquote className="wiki-quote">
          <Blocks blocks={block.blocks} context={context} />
        </blockquote>
      );

    case "list":
      return <List kind={block.kind} items={block.items} context={context} />;

    case "indent":
      return (
        <div className="wiki-indent">
          <Blocks blocks={block.blocks} context={context} />
        </div>
      );

    case "table":
      return <Table table={block.table} context={context} />;
  }
}

/** 순서 리스트의 모양은 클래스로, 시작 번호는 `start`로 준다. */
function List({
  kind,
  items,
  context,
}: {
  kind: ListKind;
  items: { startNumber: number | null; blocks: RenderBlock[] }[];
  context: RenderContext;
}): ReactNode {
  const children = items.map((item, index) => (
    <li key={index}>
      <Blocks blocks={item.blocks} context={context} />
    </li>
  ));
  if (kind === "unordered") {
    return <ul className={listClass[kind]}>{children}</ul>;
  }
  // 첫 항목의 재지정 번호가 곧 리스트의 시작 번호다.
  return (
    <ol className={listClass[kind]} start={items[0]?.startNumber ?? 1}>
      {children}
    </ol>
  );
}
