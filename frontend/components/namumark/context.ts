import type { RenderedFootnote } from "@/lib/namumark/RenderedFootnote";
import type { RenderTree } from "@/lib/namumark/RenderTree";
import type { TableOfContentsEntry } from "@/lib/namumark/TableOfContentsEntry";

/**
 * 렌더러가 트리 바깥에서 알아야 하는 것들.
 *
 * React context를 쓰지 않는다 — 서버 컴포넌트는 context를 읽지 못하는데, 이 렌더러는
 * 문서 보기(서버)와 편집 미리보기(브라우저) 양쪽에서 같은 코드로 돌아야 한다.
 * 그래서 평범한 prop으로 엮는다.
 */
export type RenderContext = {
  /** 문서 전체 목차. `[목차]` 자리가 이걸 그대로 그린다. */
  tableOfContents: readonly TableOfContentsEntry[];
  /** 문서의 모든 각주. `[각주]` 자리가 인덱스로 찾아 쓴다. */
  footnotes: readonly RenderedFootnote[];
  /**
   * 참조 번호 → 그 각주. 참조 자리의 미리보기가 내용을 꺼낸다.
   *
   * 라벨이 아니라 번호로 엮는다 — 라벨은 `[각주]` 방출 구간 안에서만 유일해서,
   * `[각주]`가 여러 번 나오면 라벨 `1`이 다시 등장한다.
   */
  footnoteByReference: ReadonlyMap<number, RenderedFootnote>;
  /**
   * 각주 미리보기를 열 수 있는 자리인지. 미리보기 **안**에서는 꺼진다 — 각주가 자기를
   * 참조하면(`[*A 앞[*A]뒤]`) 끝없이 겹쳐 들기 때문이다. 사전을 비우는 대신 이 스위치를
   * 두는 것은, 안쪽 참조도 제 각주로 가는 앵커만은 옳게 걸 수 있어야 해서다.
   */
  footnotePreviews: boolean;
  /**
   * 절 제목의 편집 링크가 가리킬 곳. 편집기 미리보기처럼 편집할 대상이 이미
   * 열려 있는 자리에서는 비어, 제목 줄에 편집이 서지 않는다.
   */
  editPath: string | null;
  /**
   * 이미 링크 안쪽인지. 링크 표시글에 이미지가 들어가면 그 이미지가 다시 링크를
   * 세울 수 있는데, `<a>` 안의 `<a>`는 브라우저가 트리를 고쳐 hydration이 어긋난다.
   */
  insideLink: boolean;
};

/**
 * 트리 하나를 그릴 맥락을 만든다.
 *
 * 목차와 각주 내용은 트리가 아니라 [`RenderTree`]의 최상위 목록이 소유하므로, 여기서
 * 트리를 훑을 일이 없다 — 참조 번호 사전만 그 목록에서 평평하게 만든다.
 */
export function renderContext(
  tree: RenderTree,
  editPath: string | null,
): RenderContext {
  const footnoteByReference = new Map<number, RenderedFootnote>();
  for (const footnote of tree.footnotes) {
    for (const number of footnote.referenceNumbers) {
      footnoteByReference.set(number, footnote);
    }
  }
  return {
    tableOfContents: tree.tableOfContents,
    footnotes: tree.footnotes,
    footnoteByReference,
    footnotePreviews: true,
    editPath,
    insideLink: false,
  };
}

/** 미리보기 안에서 쓸 맥락. 미리보기를 다시 열지 않는다 — 이유는 `footnotePreviews` 참고. */
export function withoutPreviews(context: RenderContext): RenderContext {
  return { ...context, footnotePreviews: false };
}

/**
 * 각주의 복귀 앵커 이름. 그 각주를 처음 부른 참조 번호로 짓는다.
 *
 * 라벨로 지으면 안 된다 — 라벨은 `[각주]` 방출 구간 안에서만 유일해서, `[각주]`가 여러 번
 * 나오면 서로 다른 각주가 같은 id를 갖고 모든 참조가 첫 각주로 점프한다. 참조 번호는
 * 문서 전체에서 유일하다.
 */
export function footnoteAnchor(footnote: RenderedFootnote): string {
  return `fn-${footnote.referenceNumbers[0]}`;
}

/** 링크 표시글을 그릴 맥락. 안쪽에서 링크를 한 겹 더 세우지 않는다. */
export function insideLink(context: RenderContext): RenderContext {
  return { ...context, insideLink: true };
}
