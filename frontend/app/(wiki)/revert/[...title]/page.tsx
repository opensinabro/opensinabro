import {
  DocumentFrame,
  DocumentNotice,
} from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { RevertConfirm } from "@/components/operate/revert-confirm";
import { fetchRevertTarget } from "@/lib/api/operate";
import { routeTitle, type RevisionRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: RevisionRouteProps) {
  return { title: pageTitle(await routeTitle(params), "되돌리기") };
}

export default async function RevertPage({
  params,
  searchParams,
}: RevisionRouteProps) {
  const title = await routeTitle(params);
  const revisionId = (await searchParams).uuid;

  if (!revisionId) {
    return (
      <DocumentNotice title={title} note="되돌리기">
        <Notice>되돌릴 리비전을 역사에서 고르세요.</Notice>
      </DocumentNotice>
    );
  }

  return (
    <DocumentFrame
      title={title}
      note="되돌리기"
      noteFor={(target) => `r${target.sequence}로 되돌리기`}
      result={await fetchRevertTarget(title, revisionId)}
      denied="이 문서를 되돌릴 권한이 없습니다."
      allowed={(target) => target.may}
    >
      {(target) => (
        <>
          <Notice>
            r{target.sequence}의 내용으로 새 리비전을 남깁니다. 역사는 지워지지
            않습니다.
          </Notice>
          <RevertConfirm title={target.title} revisionId={revisionId} />
        </>
      )}
    </DocumentFrame>
  );
}
