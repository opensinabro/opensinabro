import { DocumentFrame } from "@/components/layout/document-frame";
import { Notice } from "@/components/layout/notice";
import { fetchBlame } from "@/lib/api/operate";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "기여 표시") };
}

export default async function BlamePage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);

  return (
    <DocumentFrame
      title={title}
      note="줄마다 마지막으로 손댄 사람"
      result={await fetchBlame(title)}
      denied="이 문서의 기여 표시를 볼 권한이 없습니다."
      variant="full"
    >
      {({ lines }) =>
        lines.length === 0 ? (
          <div className="px-4 pt-4 sm:px-6">
            <Notice>표시할 줄이 없습니다.</Notice>
          </div>
        ) : (
          <div className="overflow-x-auto px-4 pt-4 sm:px-6">
            <table className="w-full min-w-[520px] table-fixed border-collapse">
              <thead className="sr-only">
                <tr>
                  <th scope="col">리비전</th>
                  <th scope="col">편집자</th>
                  <th scope="col">내용</th>
                </tr>
              </thead>
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
        )
      }
    </DocumentFrame>
  );
}
