import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { RevertConfirm } from "@/components/operate/revert-confirm";
import { fetchRevertTarget } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

type RevertPageProps = DocumentRouteProps & {
  searchParams: Promise<{ uuid?: string }>;
};

export async function generateMetadata({ params }: RevertPageProps) {
  return { title: pageTitle(await routeTitle(params), "되돌리기") };
}

export default async function RevertPage({
  params,
  searchParams,
}: RevertPageProps) {
  const title = await routeTitle(params);
  const revisionId = (await searchParams).uuid;

  function frame(note: string, body: React.ReactNode) {
    return (
      <WikiPage
        header={
          <PageHeader
            title={title}
            note={note}
            actions={<DocumentActions title={title} />}
          />
        }
      >
        {body}
      </WikiPage>
    );
  }

  if (!revisionId) {
    return frame(
      "되돌리기",
      <Notice>되돌릴 리비전을 역사에서 고르세요.</Notice>,
    );
  }

  const result = await fetchRevertTarget(title, revisionId);

  if (result.kind !== "found") {
    return frame("되돌리기", <Notice>그런 리비전이 없습니다.</Notice>);
  }
  if (!result.data.may) {
    return frame(
      "되돌리기",
      <Notice>이 문서를 되돌릴 권한이 없습니다.</Notice>,
    );
  }

  return frame(
    `r${result.data.sequence}로 되돌리기`,
    <>
      <Notice>
        r{result.data.sequence}의 내용으로 새 리비전을 남깁니다. 역사는 지워지지
        않습니다.
      </Notice>
      <RevertConfirm title={result.data.title} revisionId={revisionId} />
    </>,
  );
}
