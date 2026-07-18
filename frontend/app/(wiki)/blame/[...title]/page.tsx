import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchBlame } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "기여 표시") };
}

export default async function BlamePage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchBlame(title);

  const header = (
    <PageHeader
      title={title}
      note="줄마다 마지막으로 손댄 사람"
      actions={<DocumentActions title={title} />}
    />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 문서의 기여 표시를 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const { lines } = result.data;

  if (lines.length === 0) {
    return (
      <WikiPage header={header}>
        <Notice>표시할 줄이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header} variant="full">
      <div className="px-6 pt-4">
        <table className="w-full table-fixed border-collapse">
          <tbody>
            {lines.map((line, index) => (
              <tr key={index} className="border-b border-line-soft align-top">
                <td className="text-fine w-16 py-1 pr-2 text-faint">
                  r{line.sequence}
                </td>
                <td className="text-fine w-40 py-1 pr-3 text-muted">
                  {line.author}
                </td>
                <td className="py-1">
                  {/* 원문은 접지 않고 그대로 보인다 — 긴 줄은 이 칸 안에서만 밀린다. */}
                  <pre className="m-0 overflow-x-auto font-mono text-note leading-[1.7] text-body">
                    {line.text}
                  </pre>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </WikiPage>
  );
}
