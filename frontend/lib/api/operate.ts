import type { DiffLine } from "@/components/document/diff-view";
import { encodeTitle } from "@/lib/wiki-path";
import { get } from "./fetch";

// 운영 화면의 GET은 대부분 "이 일을 할 수 있는가"만 묻는다 — 판정은 서버가 하고
// 화면은 그 답에 따라 폼을 낼지 안내를 낼지 고른다.
export type OperationPermission = {
  title: string;
  may: boolean;
};

export function fetchMovable(title: string) {
  return get<OperationPermission>(`/api/move/${encodeTitle(title)}`);
}

export function fetchDeletable(title: string) {
  return get<OperationPermission>(`/api/delete/${encodeTitle(title)}`);
}

export type BlameLine = {
  sequence: number;
  author: string;
  text: string;
};

export function fetchBlame(title: string) {
  return get<{ title: string; lines: BlameLine[] }>(
    `/api/blame/${encodeTitle(title)}`,
  );
}

export type DiffResult = {
  title: string;
  sequence: number;
  lines: DiffLine[];
};

export function fetchDiff(title: string, revisionId: string) {
  return get<DiffResult>(
    `/api/diff/${encodeTitle(title)}?uuid=${encodeURIComponent(revisionId)}`,
  );
}

export type RevertTarget = {
  title: string;
  sequence: number;
  may: boolean;
};

export function fetchRevertTarget(title: string, revisionId: string) {
  return get<RevertTarget>(
    `/api/revert/${encodeTitle(title)}?uuid=${encodeURIComponent(revisionId)}`,
  );
}

export type LicenseChoice = {
  name: string;
  displayName: string;
};

export type UploadOptions = {
  licenses: LicenseChoice[];
  mediaTypes: string[];
};

export function fetchUploadOptions() {
  return get<UploadOptions>("/api/upload");
}

export type WikiConfiguration = {
  wikiName: string;
  mainDocument: string;
  contentLicense: string;
};

export function fetchConfiguration() {
  return get<WikiConfiguration>("/api/admin/config");
}

export function fetchGrantOptions() {
  return get<{ permissions: string[] }>("/api/admin/grant");
}

export type BatchRevertTargets = {
  author: string;
  titles: string[];
};

export function fetchBatchRevertTargets(author: string) {
  return get<BatchRevertTargets>(
    `/api/admin/batch-revert?author=${encodeURIComponent(author)}`,
  );
}
