import Link from "next/link";
import { DocumentActions } from "@/components/document/document-actions";
import { DocumentEditor } from "@/components/document/document-editor";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchEditable } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "편집") };
}

export default async function EditPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchEditable(title);

  const header = (
    <PageHeader
      title={title}
      actions={<DocumentActions title={title} current="edit" />}
    />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>
          이 문서를 편집할 권한이 없습니다.{" "}
          <Link
            href={wikiPath.read(title)}
            className="text-link hover:underline"
          >
            문서로 돌아가기
          </Link>
        </Notice>
      </WikiPage>
    );
  }

  // 편집 권한이 없어도 변경안은 낼 수 있는 문서가 있다. 그 흐름은 아직 서버 화면이
  // 맡으므로 넘긴다 (docs/architecture.md).
  if (result.data.editRequestOnly) {
    return (
      <WikiPage header={header}>
        <Notice>
          이 문서를 직접 편집할 권한이 없습니다. 대신 변경안을 낼 수 있습니다.
        </Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header} variant="full">
      <DocumentEditor document={result.data} />
    </WikiPage>
  );
}
