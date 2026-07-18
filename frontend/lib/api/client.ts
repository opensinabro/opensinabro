"use client";

import { encodeTitle } from "@/lib/wiki-path";
import { postJson } from "./csrf";

export async function logOut() {
  const response = await postJson("/api/logout");
  if (!response.ok) throw new Error("로그아웃하지 못했습니다.");
}

export async function renderPreview(title: string, content: string) {
  const response = await postJson("/api/preview", { title, content });
  if (!response.ok) throw new Error("미리보기를 그리지 못했습니다.");

  const body = (await response.json()) as { html: string };
  return body.html;
}

export type SaveResult =
  | { kind: "saved" }
  | { kind: "conflict"; content: string; baseRevision: string }
  | { kind: "forbidden" };

export async function saveEdit(
  title: string,
  payload: { baseRevision: string; content: string; comment: string },
): Promise<SaveResult> {
  const response = await postJson(
    `/api/edit/${encodeTitle(title)}`,
    payload,
  );

  if (response.status === 403) return { kind: "forbidden" };
  if (response.status === 409) {
    const body = (await response.json()) as {
      content: string;
      baseRevision: string;
    };
    return { kind: "conflict", ...body };
  }
  if (!response.ok) throw new Error("저장하지 못했습니다.");

  return { kind: "saved" };
}
