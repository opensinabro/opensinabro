import { DiffView } from "@/components/document/diff-view";
import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchDiff } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

type DiffPageProps = DocumentRouteProps & {
  searchParams: Promise<{ uuid?: string }>;
};

export async function generateMetadata({ params }: DiffPageProps) {
  return { title: pageTitle(await routeTitle(params), "비교") };
}

export default async function DiffPage({
  params,
  searchParams,
}: DiffPageProps) {
  const title = await routeTitle(params);
  const revisionId = (await searchParams).uuid;

  function frame(note: string | undefined, body: React.ReactNode) {
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
      "리비전 비교",
      <Notice>비교할 리비전을 역사에서 고르세요.</Notice>,
    );
  }

  const result = await fetchDiff(title, revisionId);

  if (result.kind !== "found") {
    return frame(
      "리비전 비교",
      <Notice>그런 리비전이 없거나 볼 권한이 없습니다.</Notice>,
    );
  }

  return frame(
    `r${result.data.sequence}와 그 직전의 비교`,
    <div className="mt-4">
      <DiffView lines={result.data.lines} />
    </div>,
  );
}
