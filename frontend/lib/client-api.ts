"use client";

// 상태를 바꾸는 API는 CSRF 토큰을 헤더로 싣는다. 쿠키는 HttpOnly라 스크립트가 읽지
// 못하므로 토큰은 서버가 본문으로 따로 내준다 (docs/design/07).
let cachedToken: Promise<string> | null = null;

function csrfToken() {
  cachedToken ??= fetch("/api/csrf", { cache: "no-store" })
    .then((response) => response.json())
    .then((body: { token: string }) => body.token);
  return cachedToken;
}

async function post(path: string, payload: unknown): Promise<Response> {
  return fetch(path, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-csrf-token": await csrfToken(),
    },
    body: JSON.stringify(payload),
  });
}

export async function renderPreview(title: string, content: string) {
  const response = await post("/api/preview", { title, content });
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
) {
  const response = await post(`/api/edit/${encodeURIComponent(title)}`, payload);

  if (response.status === 403) return { kind: "forbidden" } as SaveResult;
  if (response.status === 409) {
    const body = (await response.json()) as {
      content: string;
      baseRevision: string;
    };
    return { kind: "conflict", ...body } as SaveResult;
  }
  if (!response.ok) throw new Error("저장하지 못했습니다.");

  return { kind: "saved" } as SaveResult;
}
