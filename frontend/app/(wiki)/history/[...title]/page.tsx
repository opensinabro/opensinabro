import { notFound } from "next/navigation";
import { DocumentActions } from "@/components/document/document-actions";
import { HideRevisionButton } from "@/components/document/hide-revision-button";
import {
  RevisionAction,
  RevisionLine,
  RevisionList,
} from "@/components/document/revision-line";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchHistory } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "역사") };
}

export default async function HistoryPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchHistory(title);

  if (result.kind === "missing") notFound();

  const header = (
    <PageHeader
      title={title}
      note="이 문서의 편집 기록"
      actions={<DocumentActions title={title} current="history" />}
    />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 문서의 역사를 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <RevisionList>
        {result.data.revisions.map((revision) => (
          <RevisionLine
            key={revision.id}
            revision={revision}
            actions={
              <>
                <RevisionAction href={wikiPath.rawAt(title, revision.id)}>
                  원문
                </RevisionAction>
                <RevisionAction href={wikiPath.diff(title, revision.id)}>
                  비교
                </RevisionAction>
                <RevisionAction href={wikiPath.revert(title, revision.id)}>
                  되돌리기
                </RevisionAction>
                {result.data.mayHideRevision && (
                  <HideRevisionButton
                    title={title}
                    revisionId={revision.id}
                    hidden={revision.hidden}
                  />
                )}
              </>
            }
          />
        ))}
      </RevisionList>
    </WikiPage>
  );
}
