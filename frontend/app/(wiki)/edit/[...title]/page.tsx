import Link from "next/link";
import { DocumentEditor } from "@/components/document-editor";
import { fetchEditable } from "@/lib/api";

type PageProps = {
  params: Promise<{ title: string[] }>;
};

function joinTitle(segments: string[]) {
  return segments.map(decodeURIComponent).join("/");
}

export async function generateMetadata({ params }: PageProps) {
  const { title } = await params;
  return { title: `${joinTitle(title)} (편집) - 오픈시나브로` };
}

export default async function EditPage({ params }: PageProps) {
  const title = joinTitle((await params).title);
  const result = await fetchEditable(title);

  if (result.kind !== "found") {
    return (
      <article className="px-6 py-5 xl:col-span-2">
        <h1 className="m-0 text-2xl font-extrabold tracking-tight text-ink">
          {title}
        </h1>
        <p className="mt-3 text-muted">
          이 문서를 편집할 권한이 없습니다.{" "}
          <Link href={`/w/${title}`} className="text-link hover:underline">
            문서로 돌아가기
          </Link>
        </p>
      </article>
    );
  }

  // 편집 권한이 없어도 변경안은 낼 수 있는 문서가 있다. 그 흐름은 아직 서버 화면이
  // 맡으므로 넘긴다 (docs/design/07 M7).
  if (result.data.editRequestOnly) {
    return (
      <article className="px-6 py-5 xl:col-span-2">
        <h1 className="m-0 text-2xl font-extrabold tracking-tight text-ink">
          {title}
        </h1>
        <p className="mt-3 text-muted">
          이 문서를 직접 편집할 권한이 없습니다. 대신 변경안을 낼 수 있습니다.
        </p>
      </article>
    );
  }

  return <DocumentEditor document={result.data} />;
}
