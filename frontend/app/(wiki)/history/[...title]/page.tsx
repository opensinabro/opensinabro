import { HideRevisionButton } from "@/components/document/hide-revision-button";
import {
  RevisionAction,
  RevisionLine,
  RevisionList,
} from "@/components/document/revision-line";
import { DocumentFrame } from "@/components/layout/document-frame";
import { fetchHistory } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "역사") };
}

export default async function HistoryPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  return (
    <DocumentFrame
      title={title}
      note="이 문서의 편집 기록"
      tab="history"
      result={await fetchHistory(title)}
      denied="이 문서의 역사를 볼 권한이 없습니다."
    >
      {(history) => (
        <RevisionList>
          {history.revisions.map((revision) => (
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
                  {history.mayHideRevision && (
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
      )}
    </DocumentFrame>
  );
}
