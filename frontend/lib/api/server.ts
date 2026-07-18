import { encodeTitle } from "@/lib/wiki-path";
import { get, type Fetched } from "./fetch";
import type {
  BacklinkView,
  DocumentView,
  EditView,
  HistoryView,
  RecentChange,
  SessionView,
} from "./types";

export type { Fetched };

export async function fetchSession(): Promise<SessionView> {
  const result = await get<SessionView>("/api/session");
  if (result.kind !== "found") throw new Error("세션을 읽지 못했습니다.");
  return result.data;
}

export type DocumentFetched =
  | Fetched<DocumentView>
  | { kind: "redirect"; target: string };

export async function fetchDocument(title: string): Promise<DocumentFetched> {
  const result = await get<DocumentView & { redirect?: string }>(
    `/api/w/${encodeTitle(title)}`,
  );

  if (result.kind !== "found") return result;
  if (result.data.redirect) {
    return { kind: "redirect", target: result.data.redirect };
  }

  return result;
}

export function fetchHistory(title: string) {
  return get<HistoryView>(`/api/history/${encodeTitle(title)}`);
}

export function fetchBacklinks(title: string) {
  return get<BacklinkView>(`/api/backlink/${encodeTitle(title)}`);
}

export function fetchRecentChanges() {
  return get<RecentChange[]>("/api/recent-changes");
}

export function fetchEditable(title: string) {
  return get<EditView>(`/api/edit/${encodeTitle(title)}`);
}

export function fetchRaw(title: string) {
  return get<{ title: string; content: string }>(
    `/api/raw/${encodeTitle(title)}`,
  );
}
