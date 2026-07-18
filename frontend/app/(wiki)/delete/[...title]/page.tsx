import { DocumentFrame } from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { DeleteForm } from "@/components/operate/delete-form";
import { fetchDeletable } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "삭제") };
}

export default async function DeletePage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  return (
    <DocumentFrame
      title={title}
      note="문서 삭제"
      result={await fetchDeletable(title)}
      denied="이 문서를 지울 권한이 없습니다."
      allowed={(deletable) => deletable.may}
    >
      {(deletable) => (
        <>
          <Notice>역사는 남고 문서만 없는 상태가 됩니다.</Notice>
          <DeleteForm title={deletable.title} />
        </>
      )}
    </DocumentFrame>
  );
}
