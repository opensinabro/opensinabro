import type { DiffLine } from "@/components/document/diff-view";
import { encodeTitle } from "@/lib/wiki-path";
import { get } from "./fetch";

export type ThreadSummary = {
  id: string;
  topic: string;
  status: string;
  statusLabel: string;
  createdAt: string;
};

export type DocumentThreadsView = {
  title: string;
  threads: ThreadSummary[];
  mayCreate: boolean;
};

// 관리 조작이 남긴 발언은 `content` 대신 바뀐 값 하나만 싣는다 — 문장은 화면이 만든다.
export type ThreadComment = {
  sequence: number;
  kind: string;
  author: string;
  content: string;
  detail: string | null;
  adminMarked: boolean;
  hidden: boolean;
  createdAt: string;
};

export type ThreadView = {
  id: string;
  topic: string;
  title: string;
  status: string;
  statusLabel: string;
  comments: ThreadComment[];
  mayComment: boolean;
  mayModerate: boolean;
};

export type RecentThreadSummary = ThreadSummary & { title: string };

export type EditRequestSummary = {
  id: string;
  title: string;
  author: string;
  comment: string;
};

export type EditRequestView = {
  id: string;
  title: string;
  author: string;
  comment: string;
  status: string;
  statusLabel: string;
  createdAt: string;
  diff: DiffLine[];
  mayReview: boolean;
};

export function fetchDocumentThreads(title: string) {
  return get<DocumentThreadsView>(`/api/discuss/${encodeTitle(title)}`);
}

export function fetchThread(id: string) {
  return get<ThreadView>(`/api/thread/${id}`);
}

export function fetchRecentDiscussions(status?: string) {
  const query = status ? `?status=${encodeURIComponent(status)}` : "";
  return get<{ threads: RecentThreadSummary[] }>(
    `/api/recent-discussions${query}`,
  );
}

export function fetchEditRequests() {
  return get<{ requests: EditRequestSummary[] }>("/api/edit-requests");
}

export function fetchEditRequest(id: string) {
  return get<EditRequestView>(`/api/edit-request/${id}`);
}
