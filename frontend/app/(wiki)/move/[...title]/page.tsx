import { DocumentFrame } from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { MoveForm } from "@/components/operate/move-form";
import { fetchMovable } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "이동") };
}

export default async function MovePage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  return (
    <DocumentFrame
      title={title}
      note="문서 이동"
      result={await fetchMovable(title)}
      denied="이 문서를 옮길 권한이 없습니다."
      allowed={(movable) => movable.may}
    >
      {(movable) => (
        <>
          <Notice>
            제목을 옮기면 문서를 가리키던 링크도 새 제목을 따라갑니다.
          </Notice>
          <MoveForm title={movable.title} />
        </>
      )}
    </DocumentFrame>
  );
}
