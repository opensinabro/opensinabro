import { NewThreadForm } from "@/components/discussion/new-thread-form";
import { ThreadLine, ThreadList } from "@/components/discussion/thread-list";
import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchDocumentThreads } from "@/lib/api/discussion";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "토론") };
}

export default async function DiscussPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchDocumentThreads(title);

  const header = (
    <PageHeader
      title={title}
      note="이 문서에 열린 토론"
      actions={<DocumentActions title={title} current="discuss" />}
    />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 문서의 토론을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const { threads, mayCreate } = result.data;

  return (
    <WikiPage header={header}>
      {threads.length === 0 ? (
        <Notice>아직 토론이 없습니다.</Notice>
      ) : (
        <ThreadList>
          {threads.map((thread) => (
            <ThreadLine key={thread.id} {...thread} />
          ))}
        </ThreadList>
      )}

      {mayCreate ? (
        <NewThreadForm title={title} />
      ) : (
        <Notice>토론을 열 권한이 없습니다.</Notice>
      )}
    </WikiPage>
  );
}
