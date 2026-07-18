import Link from "next/link";
import { notFound } from "next/navigation";
import { EditRequestReview } from "@/components/discussion/edit-request-review";
import { DiffView } from "@/components/document/diff-view";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { Section } from "@/components/ui/section";
import { fetchEditRequest } from "@/lib/api/discussion";
import { formatMoment } from "@/lib/format";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";

type EditRequestRouteProps = { params: Promise<{ id: string }> };

export async function generateMetadata({ params }: EditRequestRouteProps) {
  const result = await fetchEditRequest((await params).id);
  return {
    title: pageTitle(
      result.kind === "found" ? result.data.title : "편집요청",
      "편집요청",
    ),
  };
}

export default async function EditRequestPage({
  params,
}: EditRequestRouteProps) {
  const { id } = await params;
  const result = await fetchEditRequest(id);

  if (result.kind === "missing") notFound();

  if (result.kind !== "found") {
    return (
      <WikiPage header={<PageHeader title="편집요청" />}>
        <Notice>이 편집요청을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const request = result.data;

  const header = (
    <PageHeader
      title={request.title}
      note={`${request.author} · ${request.statusLabel} · ${formatMoment(request.createdAt)}`}
      actions={
        <Link
          href={wikiPath.read(request.title)}
          className={linkStyle({ size: "ui" })}
        >
          문서 보기
        </Link>
      }
    />
  );

  return (
    <WikiPage header={header}>
      {request.comment && (
        <p className="text-note mt-0 mb-4 text-body">{request.comment}</p>
      )}

      <Section label="변경 내용">
        <DiffView lines={request.diff} />
      </Section>

      {request.mayReview && <EditRequestReview id={request.id} />}
    </WikiPage>
  );
}
