import Link from "next/link";
import { formatMoment } from "@/lib/format";
import { wikiPath } from "@/lib/wiki-path";

export function ThreadList({ children }: { children: React.ReactNode }) {
  return <ul className="m-0 list-none p-0">{children}</ul>;
}

export function ThreadLine({
  id,
  topic,
  statusLabel,
  createdAt,
  document,
}: {
  id: string;
  topic: string;
  statusLabel: string;
  createdAt: string;
  /** 최근 토론처럼 어느 문서의 토론인지 밝혀야 하는 목록에서만 채운다. */
  document?: string;
}) {
  return (
    <li className="text-list flex flex-wrap items-baseline gap-x-2.5 gap-y-1 border-b border-line-soft py-2.5">
      <Link
        href={wikiPath.discussThread(id)}
        className="text-link hover:underline"
      >
        {topic}
      </Link>
      {document && (
        <Link
          href={wikiPath.read(document)}
          className="text-muted hover:underline"
        >
          {document}
        </Link>
      )}
      <span className="rounded bg-accent-wash px-2 py-0.5 text-label font-semibold text-accent-deep">
        {statusLabel}
      </span>
      <span className="text-faint">{formatMoment(createdAt)}</span>
    </li>
  );
}
