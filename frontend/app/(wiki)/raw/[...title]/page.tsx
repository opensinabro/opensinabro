import { DocumentFrame } from "@/components/layout/document-frame";
import { fetchRaw } from "@/lib/api/server";
import { routeTitle, type RevisionRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";

export async function generateMetadata({ params }: RevisionRouteProps) {
  return { title: pageTitle(await routeTitle(params), "원문") };
}

export default async function RawPage({
  params,
  searchParams,
}: RevisionRouteProps) {
  const title = await routeTitle(params);
  const { uuid } = await searchParams;

  return (
    <DocumentFrame
      title={title}
      note="문서의 나무마크 원문"
      noteFor={(raw) =>
        raw.revision === null
          ? "문서의 나무마크 원문"
          : `r${raw.revision} 시점의 나무마크 원문`
      }
      result={await fetchRaw(title, uuid)}
      denied="이 문서를 읽을 권한이 없습니다."
    >
      {(raw) => (
        <pre className="m-0 overflow-x-auto rounded border border-line bg-ground-sub px-3 py-2.5 font-mono text-note leading-[1.8] text-body">
          {raw.content}
        </pre>
      )}
    </DocumentFrame>
  );
}
