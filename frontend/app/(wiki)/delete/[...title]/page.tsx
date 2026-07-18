import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { DeleteForm } from "@/components/operate/delete-form";
import { fetchDeletable } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "삭제") };
}

export default async function DeletePage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchDeletable(title);

  const header = (
    <PageHeader
      title={title}
      note="문서 삭제"
      actions={<DocumentActions title={title} />}
    />
  );

  if (result.kind !== "found" || !result.data.may) {
    return (
      <WikiPage header={header}>
        <Notice>이 문서를 지울 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <Notice>역사는 남고 문서만 없는 상태가 됩니다.</Notice>
      <DeleteForm title={result.data.title} />
    </WikiPage>
  );
}
