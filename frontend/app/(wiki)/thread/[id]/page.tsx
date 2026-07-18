import Link from "next/link";
import { notFound } from "next/navigation";
import { CommentForm } from "@/components/discussion/comment-form";
import { CommentTimeline } from "@/components/discussion/comment-timeline";
import { ThreadStatusForm } from "@/components/discussion/thread-status-form";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { Section } from "@/components/ui/section";
import { fetchThread } from "@/lib/api/discussion";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";

type ThreadRouteProps = { params: Promise<{ id: string }> };

export async function generateMetadata({ params }: ThreadRouteProps) {
  const result = await fetchThread((await params).id);
  return {
    title: pageTitle(result.kind === "found" ? result.data.topic : "토론"),
  };
}

export default async function ThreadPage({ params }: ThreadRouteProps) {
  const { id } = await params;
  const result = await fetchThread(id);

  if (result.kind === "missing") notFound();

  if (result.kind !== "found") {
    return (
      <WikiPage header={<PageHeader title="토론" />}>
        <Notice>이 토론을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const thread = result.data;

  const header = (
    <PageHeader
      title={thread.topic}
      note={`${thread.title} · ${thread.statusLabel}`}
      actions={
        <Link
          href={wikiPath.discuss(thread.title)}
          className={linkStyle({ size: "ui" })}
        >
          문서의 토론 목록
        </Link>
      }
    />
  );

  const aside = (
    <Section label="문서">
      <Link
        href={wikiPath.read(thread.title)}
        className={linkStyle({ size: "note" })}
      >
        {thread.title}
      </Link>
    </Section>
  );

  return (
    <WikiPage header={header} aside={aside}>
      {thread.comments.length === 0 ? (
        <Notice>아직 발언이 없습니다.</Notice>
      ) : (
        <CommentTimeline comments={thread.comments} />
      )}

      {thread.mayComment ? (
        <CommentForm threadId={thread.id} />
      ) : (
        <Notice>이 토론에 발언할 수 없습니다.</Notice>
      )}

      {thread.mayModerate && (
        <ThreadStatusForm threadId={thread.id} status={thread.status} />
      )}
    </WikiPage>
  );
}
