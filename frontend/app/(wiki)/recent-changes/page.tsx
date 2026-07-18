import Link from "next/link";
import { PageHeader } from "@/components/page-header";
import { fetchRecentChanges } from "@/lib/api";
import { formatMoment, revisionKindLabel } from "@/lib/format";

export const metadata = { title: "최근 변경 - 오픈시나브로" };

export default async function RecentChangesPage() {
  const result = await fetchRecentChanges();
  const changes = result.kind === "found" ? result.data : [];

  return (
    <article className="min-w-0 pb-7">
      <PageHeader title="최근 변경" note="최근에 편집된 문서" />

      {changes.length === 0 ? (
        <p className="px-6 pt-4 text-muted">아직 변경 기록이 없습니다.</p>
      ) : (
        <ol className="m-0 max-w-[900px] list-none px-6 pt-4">
          {changes.map((change) => (
            <li
              key={change.revision.id}
              className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1 border-b border-line-soft py-2.5 text-[13.5px]"
            >
              <Link
                href={`/w/${change.title}`}
                className="font-medium text-link hover:underline"
              >
                {change.title}
              </Link>
              <span className="rounded bg-accent-wash px-2 py-0.5 text-[11px] font-semibold text-accent-deep">
                {revisionKindLabel(change.revision.kind)}
              </span>
              <span className="tabular-nums text-muted">
                r{change.revision.sequence}
              </span>
              <span className="text-muted">{change.revision.author}</span>
              <span className="text-faint">
                {formatMoment(change.revision.createdAt)}
              </span>
              {change.revision.comment && (
                <span className="text-body">{change.revision.comment}</span>
              )}
              <Link
                href={`/diff/${change.title}?uuid=${change.revision.id}`}
                className="ml-auto text-link hover:underline"
              >
                비교
              </Link>
            </li>
          ))}
        </ol>
      )}
    </article>
  );
}
