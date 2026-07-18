import { notFound } from "next/navigation";
import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchRaw } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "원문") };
}

export default async function RawPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchRaw(title);

  if (result.kind === "missing") notFound();

  const header = (
    <PageHeader
      title={title}
      note="문서의 나무마크 원문"
      actions={<DocumentActions title={title} />}
    />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 문서를 읽을 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <pre className="m-0 overflow-x-auto rounded border border-line bg-ground-sub px-3 py-2.5 font-mono text-note leading-[1.8] text-body">
        {result.data.content}
      </pre>
    </WikiPage>
  );
}
