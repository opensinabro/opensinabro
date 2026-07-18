import Link from "next/link";
import { DocumentEditor } from "@/components/document/document-editor";
import { DocumentFrame } from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { linkStyle } from "@/components/ui/link";
import { fetchEditable } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "편집") };
}

export default async function EditPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  return (
    <DocumentFrame
      title={title}
      tab="edit"
      result={await fetchEditable(title)}
      denied="이 문서를 편집할 권한이 없습니다."
      variant="full"
    >
      {(editable) =>
        // 편집 권한이 없어도 변경안은 낼 수 있는 문서가 있다. 그 흐름은 아직 서버
        // 화면이 맡으므로 넘긴다 (docs/architecture.md).
        editable.editRequestOnly ? (
          <div className="px-4 pt-4 sm:px-6">
            <Notice>
              이 문서를 직접 편집할 권한이 없습니다. 대신 변경안을 낼 수
              있습니다.{" "}
              <Link href={wikiPath.read(title)} className={linkStyle()}>
                문서로 돌아가기
              </Link>
            </Notice>
          </div>
        ) : (
          <DocumentEditor document={editable} />
        )
      }
    </DocumentFrame>
  );
}
