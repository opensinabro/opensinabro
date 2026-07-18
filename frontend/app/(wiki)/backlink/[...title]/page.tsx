import Link from "next/link";
import { notFound } from "next/navigation";
import { PageHeader } from "@/components/page-header";
import { fetchBacklinks } from "@/lib/api";

type PageProps = {
  params: Promise<{ title: string[] }>;
};

function joinTitle(segments: string[]) {
  return segments.map(decodeURIComponent).join("/");
}

export async function generateMetadata({ params }: PageProps) {
  const { title } = await params;
  return { title: `${joinTitle(title)} (역링크) - 오픈시나브로` };
}

export default async function BacklinkPage({ params }: PageProps) {
  const title = joinTitle((await params).title);
  const result = await fetchBacklinks(title);

  if (result.kind === "missing") notFound();
  if (result.kind === "forbidden") {
    return (
      <article className="px-6 py-5">
        <p className="text-muted">이 문서의 역링크를 볼 권한이 없습니다.</p>
      </article>
    );
  }

  const { entries } = result.data;

  return (
    <article className="min-w-0 pb-7">
      <PageHeader
        title={title}
        note="이 문서를 링크하거나 포함하는 문서"
        actions={
          <Link
            href={`/w/${title}`}
            className="rounded border border-line px-2.5 py-1 text-[13px] text-body hover:border-accent hover:text-accent-deep"
          >
            문서로
          </Link>
        }
      />

      {entries.length === 0 ? (
        <p className="px-6 pt-4 text-muted">
          이 문서를 가리키는 문서가 아직 없습니다.
        </p>
      ) : (
        <ul className="m-0 max-w-[900px] list-none px-6 pt-4">
          {entries.map((entry) => (
            <li
              key={`${entry.kind}:${entry.title}`}
              className="flex items-baseline gap-2.5 border-b border-line-soft py-2 text-[13.5px]"
            >
              <Link
                href={`/w/${entry.title}`}
                className="text-link hover:underline"
              >
                {entry.title}
              </Link>
              <span className="text-faint">{entry.kind}</span>
            </li>
          ))}
        </ul>
      )}
    </article>
  );
}
