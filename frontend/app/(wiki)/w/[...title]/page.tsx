import { notFound, redirect } from "next/navigation";
import { DocumentActions } from "@/components/document-actions";
import { DocumentInfo } from "@/components/document-info";
import { fetchDocument } from "@/lib/api";

type PageProps = {
  params: Promise<{ title: string[] }>;
};

// 제목은 하위 문서 관습 때문에 `/`를 품는다. catch-all 조각을 다시 이어 붙여야
// `상위/하위`가 한 제목으로 복원된다 (docs/design/07 URL 설계).
function joinTitle(segments: string[]) {
  return segments.map(decodeURIComponent).join("/");
}

export async function generateMetadata({ params }: PageProps) {
  const { title } = await params;
  return { title: `${joinTitle(title)} - 오픈시나브로` };
}

export default async function DocumentPage({ params }: PageProps) {
  const title = joinTitle((await params).title);
  const result = await fetchDocument(title);

  if (result.kind === "redirect") {
    redirect(`/w/${result.target}?from=${encodeURIComponent(title)}`);
  }

  if (result.kind === "forbidden") {
    return (
      <article className="px-6 py-5">
        <h1 className="m-0 text-2xl font-extrabold tracking-tight text-ink">
          {title}
        </h1>
        <p className="mt-3 text-muted">이 문서를 읽을 권한이 없습니다.</p>
      </article>
    );
  }

  if (result.kind === "missing") notFound();

  const { document } = result;

  return (
    <>
      <article className="min-w-0 pb-7">
        <header className="flex items-end justify-between gap-4 px-6 pt-4">
          <h1 className="m-0 text-[27px] font-extrabold tracking-tight text-ink">
            {document.title}
          </h1>
          <DocumentActions title={title} current="read" />
        </header>
        <div className="border-b border-line" />

        {/* 3열 중 가운데가 가변이라 초광폭 화면에서 한 줄이 지나치게 길어진다. */}
        <div className="max-w-[900px] px-6 pt-3.5">
          {document.revision && (
            <p className="mb-4 text-[12.5px] text-faint">
              <b className="font-semibold text-muted">
                r{document.revision.sequence}
              </b>{" "}
              · {document.revision.author} 편집
            </p>
          )}

          {/* 본문 HTML은 backend-namuwiki가 방출한 것을 그대로 싣는다. 셸은 이 안을
              건드리지 않는다 — 파리티 스냅샷의 대상이 바로 이 마크업이다. */}
          <div
            className="wiki-content"
            dangerouslySetInnerHTML={{ __html: document.html }}
          />

          <div className="mt-8 border-t border-line pt-4 xl:hidden">
            <DocumentInfo document={document} />
          </div>
        </div>
      </article>

      <aside className="hidden border-l border-line px-4 py-4 xl:block">
        <div className="sticky top-4">
          <DocumentInfo document={document} />
        </div>
      </aside>
    </>
  );
}
