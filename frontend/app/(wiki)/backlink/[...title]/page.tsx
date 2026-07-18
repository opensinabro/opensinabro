import Link from "next/link";
import { notFound } from "next/navigation";
import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchBacklinks } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "역링크") };
}

export default async function BacklinkPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchBacklinks(title);

  if (result.kind === "missing") notFound();

  // 역링크는 탭에 없는 도구 화면이라 어느 탭도 현재가 아니다 — 탭 줄은 그대로 걸어
  // 돌아가는 길이 다른 문서 화면과 같은 자리에 있게 한다.
  const header = (
    <PageHeader
      title={title}
      note="이 문서를 링크하거나 포함하는 문서"
      actions={<DocumentActions title={title} />}
    />
  );

  if (result.kind === "unauthorized") {
    return (
      <WikiPage header={header}>
        <Notice>로그인해야 볼 수 있습니다.</Notice>
      </WikiPage>
    );
  }

  if (result.kind === "forbidden") {
    return (
      <WikiPage header={header}>
        <Notice>이 문서의 역링크를 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const { entries } = result.data;

  return (
    <WikiPage header={header}>
      {entries.length === 0 ? (
        <Notice>이 문서를 가리키는 문서가 아직 없습니다.</Notice>
      ) : (
        <ul className="m-0 list-none p-0">
          {entries.map((entry) => (
            <li
              key={`${entry.kind}:${entry.title}`}
              className="text-list flex items-baseline gap-2.5 border-b border-line-soft py-2"
            >
              <Link
                href={wikiPath.read(entry.title)}
                className="text-link hover:underline"
              >
                {entry.title}
              </Link>
              <span className="text-faint">{entry.kindLabel}</span>
            </li>
          ))}
        </ul>
      )}
    </WikiPage>
  );
}
