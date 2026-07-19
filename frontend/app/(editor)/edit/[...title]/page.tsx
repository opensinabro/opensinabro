import { notFound } from "next/navigation";
import { DocumentEditor } from "@/components/document/document-editor";
import { Link } from "@/components/layout/link";
import { buttonStyle } from "@/components/ui/button";
import { fetchEditable } from "@/lib/api/server";
import { routeTitle, type DocumentRouteProps } from "@/lib/document-route";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export async function generateMetadata({ params }: DocumentRouteProps) {
  return { title: pageTitle(await routeTitle(params), "편집") };
}

// 편집할 수 없을 때의 화면. 셸이 없으므로 안내도 스스로 서야 한다 — 빈 화면 한가운데에
// 사정과 돌아갈 길만 둔다. 여기서 문서 탭이나 내비를 다시 그리면 편집기가 셸을 쓰지
// 않기로 한 이유가 무너진다.
function CannotEdit({
  title,
  reason,
  children,
}: {
  title: string;
  reason: string;
  children?: React.ReactNode;
}) {
  return (
    <main
      id="content"
      className="flex flex-1 flex-col items-center justify-center gap-4 px-6 text-center"
    >
      <h1 className="text-title m-0 font-extrabold tracking-tight text-ink">
        {title}
      </h1>
      <p className="text-ui m-0 max-w-[46ch] text-muted">{reason}</p>
      <div className="flex flex-wrap items-center justify-center gap-2">
        {children}
        <Link href={wikiPath.read(title)} className={buttonStyle()}>
          문서로 돌아가기
        </Link>
      </div>
    </main>
  );
}

export default async function EditPage({ params }: DocumentRouteProps) {
  const title = await routeTitle(params);
  const result = await fetchEditable(title);

  if (result.kind === "missing") notFound();

  if (result.kind === "unauthorized") {
    return (
      <CannotEdit title={title} reason="로그인해야 편집할 수 있습니다.">
        <Link href="/login" className={buttonStyle({ tone: "primary" })}>
          로그인하기
        </Link>
      </CannotEdit>
    );
  }

  if (result.kind === "forbidden") {
    return (
      <CannotEdit title={title} reason="이 문서를 편집할 권한이 없습니다." />
    );
  }

  // 편집 권한이 없어도 변경안은 낼 수 있는 문서가 있다. 그 흐름은 아직 서버 화면이
  // 맡으므로 넘긴다 (docs/architecture.md).
  if (result.data.editRequestOnly) {
    return (
      <CannotEdit
        title={title}
        reason="이 문서를 직접 편집할 권한이 없습니다. 대신 변경안을 낼 수 있습니다."
      />
    );
  }

  return <DocumentEditor document={result.data} />;
}
