import { redirect } from "next/navigation";
import { DocumentInfo } from "@/components/document/document-info";
import { TitleList } from "@/components/document/title-list";
import { DocumentFrame } from "@/components/layout/document-frame";
import { Section } from "@/components/ui/section";
import { fetchCategoryMembers } from "@/lib/api/special";
import { fetchDocument } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { encodeTitle, wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params)) };
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
      aside={(document) => <DocumentInfo document={document} />}
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
            {document.revision && (
              <p className="text-note mb-4 text-faint">
                <b className="font-semibold text-muted">
                  r{document.revision.sequence}
                </b>{" "}
                · {document.revision.author} 편집
              </p>
            )}

            {/* 본문 HTML은 backend-namuwiki가 방출한 것을 그대로 싣는다. 셸은 이 안을
                건드리지 않는다 — 파리티 스냅샷의 대상이 바로 이 마크업이다. */}
            <div
              className="wiki-content"
              dangerouslySetInnerHTML={{ __html: document.html }}
            />

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
