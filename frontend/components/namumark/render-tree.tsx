import { Fragment, type ReactNode } from "react";
import { Link } from "@/components/layout/link";
import type { RenderBlock } from "@/lib/namumark/RenderBlock";
import type { RenderTree as Tree } from "@/lib/namumark/RenderTree";
import { encodeTitle } from "@/lib/wiki-path";
import { Blocks, HeadingLine, type HeadingBlock } from "./blocks";
import { renderContext } from "./context";

/**
 * 렌더 트리 하나를 본문으로 그린다.
 *
 * 서버 컴포넌트도 클라이언트 컴포넌트도 아니다 — 문서 보기는 서버에서, 편집 미리보기는
 * 브라우저에서 같은 트리를 그리므로 어느 쪽에서도 돌아야 한다. 그래서 이 아래 어디에도
 * 훅이나 브라우저 API가 없다. 예외는 문서로 가는 링크 하나뿐이고, 그것도 훅을 자기
 * 경계 안에 가둔 컴포넌트라 양쪽에서 그대로 돈다 (DocumentLink).
 */
export function RenderTree({
  tree,
  editPath,
}: {
  tree: Tree;
  /** 절 제목마다 걸 편집 링크. 편집기 미리보기는 주지 않는다. */
  editPath?: string;
}): ReactNode {
  const context = renderContext(tree, editPath ?? null);
  return (
    <div className="wiki-content">
      {tree.categories.length > 0 && (
        <Categories categories={tree.categories} />
      )}
      {headingSections(tree.blocks).map((section, index) =>
        section.heading ? (
          // 절은 여닫이다. 접힘 여부는 문법이 정하고(`== 제목 ==` 앞의 `#`),
          // 그 뒤로는 읽는 사람이 정한다.
          <details
            key={index}
            className="wiki-section"
            id={`s-${section.heading.number}`}
            open={!section.heading.folded}
          >
            <summary className="wiki-heading">
              <HeadingLine
                block={section.heading}
                foldable
                context={context}
              />
            </summary>
            <div className="wiki-heading-content">
              <Blocks blocks={section.blocks} context={context} />
            </div>
          </details>
        ) : (
          <Fragment key={index}>
            <Blocks blocks={section.blocks} context={context} />
          </Fragment>
        ),
      )}
    </div>
  );
}

type HeadingSection = {
  heading: HeadingBlock | null;
  blocks: RenderBlock[];
};

/**
 * 헤딩마다 그 뒤 블록을 한 구역으로 묶는다.
 *
 * 구역은 수준과 무관하게 헤딩마다 닫고 다시 연다 — 나무위키는 하위 문단을 상위 문단
 * 안에 넣지 않는다. 문서 끝 각주는 마지막 구역 **바깥**, 문서 레벨에 온다.
 */
function headingSections(blocks: RenderBlock[]): HeadingSection[] {
  const sections: HeadingSection[] = [{ heading: null, blocks: [] }];

  for (const block of blocks) {
    if (block.type === "heading") {
      sections.push({ heading: block, blocks: [] });
      continue;
    }
    if (isTrailingFootnotes(block)) {
      sections.push({ heading: null, blocks: [block] });
      continue;
    }
    sections[sections.length - 1].blocks.push(block);
  }

  return sections.filter(
    (section) => section.heading !== null || section.blocks.length > 0,
  );
}

function isTrailingFootnotes(block: RenderBlock): boolean {
  return (
    block.type === "paragraph" &&
    block.content.length === 1 &&
    block.content[0].type === "footnoteSection"
  );
}

function Categories({ categories }: { categories: string[] }): ReactNode {
  return (
    <div className="wiki-categories">
      분류{" "}
      {categories.map((category, index) => (
        <Fragment key={category}>
          {index > 0 && " · "}
          <Link href={`/w/${encodeTitle(`분류:${category}`)}`} prefetch={false}>
            {category}
          </Link>
        </Fragment>
      ))}
    </div>
  );
}
