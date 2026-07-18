import { headers } from "next/headers";

// 서버 컴포넌트는 프록시를 거치지 않고 axum을 직접 부른다 — 브라우저 → axum →
// Next → axum 왕복을 한 번 줄인다 (docs/design/07).
const internalBase =
  process.env.OPENSINABRO_INTERNAL_API ?? "http://127.0.0.1:3000";

export type RevisionSummary = {
  id: string;
  sequence: number;
  kind: string;
  author: string;
  comment: string;
  contentBytes: number;
  createdAt: string;
};

export type DocumentView = {
  title: string;
  namespace: string;
  source: string;
  html: string;
  revision: RevisionSummary | null;
  backlinkCount: number;
  threadCount: number;
};

export type DocumentResult =
  | { kind: "found"; document: DocumentView }
  | { kind: "missing" }
  | { kind: "redirect"; target: string }
  | { kind: "forbidden" };

// 요청자의 신원은 쿠키와 IP에 실려 있고, 권한 판정은 axum이 한다. 서버 컴포넌트가
// 대신 부를 때도 원래 요청의 신원이 그대로 따라가야 판정이 어긋나지 않는다.
async function forwardedHeaders(): Promise<HeadersInit> {
  const incoming = await headers();
  const forwarded: Record<string, string> = {};

  const cookie = incoming.get("cookie");
  if (cookie) forwarded.cookie = cookie;

  const forwardedFor = incoming.get("x-forwarded-for");
  if (forwardedFor) forwarded["x-forwarded-for"] = forwardedFor;

  return forwarded;
}

type Fetched<T> =
  | { kind: "found"; data: T }
  | { kind: "missing" }
  | { kind: "forbidden" };

async function get<T>(path: string): Promise<Fetched<T>> {
  const response = await fetch(`${internalBase}${path}`, {
    headers: await forwardedHeaders(),
    cache: "no-store",
  });

  if (response.status === 404) return { kind: "missing" };
  if (response.status === 403) return { kind: "forbidden" };
  if (!response.ok) {
    throw new Error(`불러오지 못했습니다 (${response.status})`);
  }

  return { kind: "found", data: (await response.json()) as T };
}

function documentPath(prefix: string, title: string) {
  return `${prefix}/${encodeURIComponent(title)}`;
}

export async function fetchDocument(title: string): Promise<DocumentResult> {
  const result = await get<DocumentView & { redirect?: string }>(
    documentPath("/api/w", title),
  );

  if (result.kind !== "found") return result;
  if (result.data.redirect) {
    return { kind: "redirect", target: result.data.redirect };
  }

  return { kind: "found", document: result.data };
}

export type HistoryView = {
  title: string;
  revisions: RevisionSummary[];
};

export function fetchHistory(title: string) {
  return get<HistoryView>(documentPath("/api/history", title));
}

export type BacklinkView = {
  title: string;
  entries: { title: string; kind: string }[];
};

export function fetchBacklinks(title: string) {
  return get<BacklinkView>(documentPath("/api/backlink", title));
}

export type RecentChange = {
  title: string;
  revision: RevisionSummary;
};

export function fetchRecentChanges() {
  return get<RecentChange[]>("/api/recent-changes");
}

export type EditView = {
  title: string;
  content: string;
  baseRevision: string;
  editRequestOnly: boolean;
};

export function fetchEditable(title: string) {
  return get<EditView>(documentPath("/api/edit", title));
}
