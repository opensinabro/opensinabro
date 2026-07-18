import Link from "next/link";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";
import { formatMoment } from "@/lib/format";
import { wikiPath } from "@/lib/wiki-path";

export function ThreadList({ children }: { children: React.ReactNode }) {
  return <Rows>{children}</Rows>;
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
    <Row>
      <Link href={wikiPath.discussThread(id)} className={linkStyle()}>
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
    </Row>
  );
}
