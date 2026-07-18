import Link from "next/link";
import { notFound } from "next/navigation";
import { DocumentActions } from "@/components/document-actions";
import { PageHeader } from "@/components/page-header";
import { fetchHistory } from "@/lib/api";
import { formatMoment, revisionKindLabel } from "@/lib/format";

type PageProps = {
  params: Promise<{ title: string[] }>;
};

function joinTitle(segments: string[]) {
  return segments.map(decodeURIComponent).join("/");
}

export async function generateMetadata({ params }: PageProps) {
  const { title } = await params;
  return { title: `${joinTitle(title)} (역사) - 오픈시나브로` };
}

export default async function HistoryPage({ params }: PageProps) {
  const title = joinTitle((await params).title);
  const result = await fetchHistory(title);

  if (result.kind === "missing") notFound();
  if (result.kind === "forbidden") {
    return (
      <article className="px-6 py-5">
        <p className="text-muted">이 문서의 역사를 볼 권한이 없습니다.</p>
      </article>
    );
  }

  return (
    <article className="min-w-0 pb-7">
      <PageHeader
        title={title}
        actions={<DocumentActions title={title} current="history" />}
      />

      <ol className="m-0 max-w-[900px] list-none px-6 pt-4">
        {result.data.revisions.map((revision) => (
          <li
            key={revision.id}
            className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1 border-b border-line-soft py-2.5 text-[13.5px]"
          >
            <span className="font-semibold tabular-nums text-ink">
              r{revision.sequence}
            </span>
            <span className="rounded bg-accent-wash px-2 py-0.5 text-[11px] font-semibold text-accent-deep">
              {revisionKindLabel(revision.kind)}
            </span>
            <span className="text-muted">{revision.author}</span>
            <span className="text-faint">{formatMoment(revision.createdAt)}</span>
            <span className="tabular-nums text-faint">
              {revision.contentBytes.toLocaleString()} B
            </span>
            {revision.comment && (
              <span className="text-body">{revision.comment}</span>
            )}
            <span className="ml-auto flex gap-2.5">
              <Link
                href={`/raw/${title}?uuid=${revision.id}`}
                className="text-link hover:underline"
              >
                원문
              </Link>
              <Link
                href={`/diff/${title}?uuid=${revision.id}`}
                className="text-link hover:underline"
              >
                비교
              </Link>
              <Link
                href={`/revert/${title}?uuid=${revision.id}`}
                className="text-link hover:underline"
              >
                되돌리기
              </Link>
            </span>
          </li>
        ))}
      </ol>
    </article>
  );
}
