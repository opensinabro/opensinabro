import { redirect } from "next/navigation";
import { DocumentToolbar } from "@/components/document/document-toolbar";
import { RenderTree } from "@/components/namumark/render-tree";
import { TitleList } from "@/components/document/title-list";
import { DocumentFrame } from "@/components/layout/document-frame";
import { Section } from "@/components/ui/section";
import { fetchCategoryMembers } from "@/lib/api/special";
import { fetchDocument } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { formatDay } from "@/lib/format";
import { plainText } from "@/lib/namumark/plain-text";
import { pageTitle } from "@/lib/site";
import { encodeTitle, wikiPath } from "@/lib/wiki-path";
import type { DocumentView } from "@/lib/api/types";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params)) };
}

// 제목 밑에는 "언제 것을 보고 있는가"만 남긴다 — 편집자와 문서 크기는 역사 화면이
// 이미 리비전별로 싣고 있고, 읽는 사람이 본문에 들어가기 전에 알아야 할 것도 아니다.
function metaLine(document: DocumentView) {
  const { revision } = document;
  if (!revision) return "";

  return `r${revision.sequence} · ${formatDay(revision.createdAt)}에 편집`;
}

export default async function DocumentPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchDocument(title);

  if (result.kind === "redirect") {
    redirect(`${wikiPath.read(result.target)}?from=${encodeTitle(title)}`);
  }

  return (
    <DocumentFrame
      title={title}
      tab="read"
      result={result}
      denied="이 문서를 읽을 권한이 없습니다."
      noteFor={metaLine}
      toolbarFor={(document) => (
        <DocumentToolbar title={title} starred={document.starred} />
      )}
      // 목차는 렌더러가 이미 구조로 내준다 — 본문 HTML을 훑어 헤딩을 다시 모으지
      // 않는다. 축은 글자만 쓰므로 인라인 트리는 여기서 평문으로 눌러 넘긴다.
      tocFor={(document) =>
        document.tree.tableOfContents.map((entry) => ({
          number: entry.number,
          depth: entry.depth,
          text: plainText(entry.title),
        }))
      }
    >
      {async (document) => {
        const members =
          document.namespace === "분류"
            ? await fetchCategoryMembers(title).then((fetched) =>
                fetched.kind === "found" ? fetched.data.members : [],
              )
            : [];

        return (
          <>
            {/* 본문은 렌더 트리에서 바로 그린다 — 서버가 보낸 HTML을 주입하지 않는다. */}
            <RenderTree tree={document.tree} editPath={wikiPath.edit(title)} />

            {/* 분류 문서의 본문은 설명글이고, 그 분류에 든 문서 목록은 본문이 아니라
                셸이 모아 붙인다 — 렌더러 출력에는 없는 자료다. */}
            {document.namespace === "분류" && (
              <div className="mt-8 border-t border-line pt-4">
                <Section label="이 분류에 속한 문서">
                  <TitleList
                    entries={members}
                    empty="이 분류에 속한 문서가 아직 없습니다."
                  />
                </Section>
              </div>
            )}
          </>
        );
      }}
    </DocumentFrame>
  );
}
