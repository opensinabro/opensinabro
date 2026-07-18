import { NewThreadForm } from "@/components/discussion/new-thread-form";
import { ThreadLine, ThreadList } from "@/components/discussion/thread-list";
import { DocumentFrame } from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { fetchDocumentThreads } from "@/lib/api/discussion";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "토론") };
}

export default async function DiscussPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  return (
    <DocumentFrame
      title={title}
      note="이 문서에 열린 토론"
      tab="discuss"
      result={await fetchDocumentThreads(title)}
      denied="이 문서의 토론을 볼 권한이 없습니다."
    >
      {({ threads, mayCreate }) => (
        <>
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
        </>
      )}
    </DocumentFrame>
  );
}
