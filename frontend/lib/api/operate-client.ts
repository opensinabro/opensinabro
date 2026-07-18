"use client";

import { encodeTitle } from "@/lib/wiki-path";
import { csrfToken, expectOk, postJson } from "./csrf";

export async function moveDocument(
  title: string,
  payload: { target: string; comment: string },
) {
  const response = await postJson(`/api/move/${encodeTitle(title)}`, payload);
  await expectOk(response, "문서를 옮기지 못했습니다.");

  const body = (await response.json()) as { title: string };
  return body.title;
}

export async function deleteDocument(title: string, comment: string) {
  const response = await postJson(`/api/delete/${encodeTitle(title)}`, {
    comment,
  });
  await expectOk(response, "문서를 지우지 못했습니다.");

  const body = (await response.json()) as { title: string };
  return body.title;
}

export async function revertDocument(title: string, revisionId: string) {
  const response = await postJson(`/api/revert/${encodeTitle(title)}`, {
    uuid: revisionId,
  });
  await expectOk(response, "되돌리지 못했습니다.");

  const body = (await response.json()) as { title: string };
  return body.title;
}

export async function uploadFile(fields: {
  file: File;
  name: string;
  license: string;
  category: string;
  description: string;
}) {
  const token = await csrfToken();
  const form = new FormData();
  form.set("file", fields.file);
  form.set("name", fields.name);
  form.set("license", fields.license);
  form.set("category", fields.category);
  form.set("description", fields.description);
  // multipart 본문은 서버가 필드로 토큰을 읽는다 — 헤더만으로는 통과하지 못한다.
  form.set("csrf_token", token);

  const response = await fetch("/api/upload", {
    method: "POST",
    headers: { "x-csrf-token": token },
    body: form,
  });
  await expectOk(response, "파일을 올리지 못했습니다.");

  const body = (await response.json()) as { title: string };
  return body.title;
}

export async function saveConfiguration(payload: {
  wikiName: string;
  mainDocument: string;
  contentLicense: string;
}) {
  const response = await postJson("/api/admin/config", payload);
  await expectOk(response, "설정을 저장하지 못했습니다.");
}

export async function changePermission(payload: {
  userName: string;
  permission: string;
  revoke: boolean;
}) {
  const response = await postJson("/api/admin/grant", payload);
  // 없는 사용자는 404로 따로 온다 — 권한 부족과 섞이지 않게 문구를 갈라 준다.
  if (response.status === 404) {
    throw new Error("그런 사용자가 없습니다.");
  }
  await expectOk(response, "권한을 바꾸지 못했습니다.");
}

export async function batchRevert(author: string) {
  const response = await postJson("/api/admin/batch-revert", { author });
  await expectOk(response, "일괄 되돌리기를 하지 못했습니다.");

  const body = (await response.json()) as { reverted: number };
  return body.reverted;
}
