import { DiffView } from "@/components/document/diff-view";
import {
  DocumentFrame,
  DocumentNotice,
} from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { fetchDiff } from "@/lib/api/operate";
import { routeTitle, type RevisionRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: RevisionRouteProps) {
  return { title: pageTitle(await routeTitle(params), "비교") };
}

export default async function DiffPage({
  params,
  searchParams,
}: RevisionRouteProps) {
  const title = await routeTitle(params);
  const revisionId = (await searchParams).uuid;

  if (!revisionId) {
    return (
      <DocumentNotice title={title} note="리비전 비교">
        <Notice>비교할 리비전을 역사에서 고르세요.</Notice>
      </DocumentNotice>
    );
  }

  return (
    <DocumentFrame
      title={title}
      note="리비전 비교"
      noteFor={(diff) => `r${diff.sequence}와 그 직전의 비교`}
      result={await fetchDiff(title, revisionId)}
      denied="그런 리비전이 없거나 볼 권한이 없습니다."
    >
      {(diff) => (
        <div className="mt-4">
          <DiffView lines={diff.lines} />
        </div>
      )}
    </DocumentFrame>
  );
}
