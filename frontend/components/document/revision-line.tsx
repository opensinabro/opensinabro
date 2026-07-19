import { Link } from "@/components/layout/link";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";
import { formatBytes, formatMoment, revisionKindLabel } from "@/lib/format";
import type { RevisionSummary } from "@/lib/api/types";

// 역사와 최근 변경은 같은 줄을 다르게 늘어놓고 있었다. 순서를 한 벌로 못박아
// 두 목록이 같은 것을 같은 자리에서 읽히게 한다 — 앞머리와 뒷단 동작만 화면이 채운다.
export function RevisionLine({
  revision,
  lead,
  actions,
}: {
  revision: RevisionSummary;
  /** 최근 변경처럼 어느 문서인지 밝혀야 하는 목록의 앞머리. */
  lead?: React.ReactNode;
  actions?: React.ReactNode;
}) {
  return (
    <Row>
      {lead}
      <span className="font-semibold tabular-nums text-ink">
        r{revision.sequence}
      </span>
      <span className="rounded bg-accent-wash px-2 py-0.5 text-badge font-semibold text-accent-deep">
        {revisionKindLabel(revision.kind)}
      </span>
      <span className="text-muted">{revision.author}</span>
      <span className="text-faint">{formatMoment(revision.createdAt)}</span>
      <span className="tabular-nums text-faint">
        {formatBytes(revision.contentBytes)}
      </span>
      {revision.hidden && (
        <span className="rounded bg-danger-wash px-2 py-0.5 text-badge font-semibold text-danger-ink">
          가려짐
        </span>
      )}
      {revision.comment && <span className="text-body">{revision.comment}</span>}
      {actions && <span className="ml-auto flex gap-2.5">{actions}</span>}
    </Row>
  );
}

export function RevisionAction({
  href,
  children,
}: {
  href: string;
  children: React.ReactNode;
}) {
  return (
    <Link href={href} className={linkStyle()}>
      {children}
    </Link>
  );
}

export function RevisionList({ children }: { children: React.ReactNode }) {
  return <Rows as="ol">{children}</Rows>;
}
