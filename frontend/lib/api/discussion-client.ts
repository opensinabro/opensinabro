"use client";

import { encodeTitle } from "@/lib/wiki-path";
import { expectOk, postJson } from "./csrf";

export async function createThread(
  title: string,
  payload: { topic: string; content: string },
) {
  const response = await postJson(`/api/discuss/${encodeTitle(title)}`, payload);
  await expectOk(response, "토론을 열지 못했습니다.");

  const body = (await response.json()) as { threadId: string };
  return body.threadId;
}

export async function addComment(threadId: string, content: string) {
  const response = await postJson(`/api/thread/${threadId}/comment`, {
    content,
  });
  // 닫히거나 중단된 토론은 발언을 받지 않는다 — 화면을 그린 뒤 상태가 바뀌었을 수 있다.
  if (response.status === 409) {
    throw new Error("이 토론은 더 이상 발언을 받지 않습니다.");
  }
  await expectOk(response, "발언을 남기지 못했습니다.");
}

export async function changeThreadStatus(threadId: string, status: string) {
  const response = await postJson(`/api/thread/${threadId}/status`, { status });
  await expectOk(response, "상태를 바꾸지 못했습니다.");
}

export async function acceptEditRequest(id: string) {
  const response = await postJson(`/api/edit-request/${id}/accept`);
  if (response.status === 409) {
    throw new Error("이미 처리된 편집요청입니다.");
  }
  await expectOk(response, "편집요청을 반영하지 못했습니다.");

  const body = (await response.json()) as { title: string };
  return body.title;
}

export async function closeEditRequest(id: string) {
  const response = await postJson(`/api/edit-request/${id}/close`);
  await expectOk(response, "편집요청을 닫지 못했습니다.");
}
