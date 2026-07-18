"use client";

// 상태를 바꾸는 API는 CSRF 토큰을 헤더로 싣는다. 쿠키는 HttpOnly라 스크립트가 읽지
// 못하므로 토큰은 서버가 본문으로 따로 내준다 (docs/architecture.md).
let cachedToken: Promise<string> | null = null;

export function csrfToken() {
  cachedToken ??= fetch("/api/csrf", { cache: "no-store" })
    .then((response) => response.json())
    .then((body: { token: string }) => body.token);
  return cachedToken;
}

export async function postJson(
  path: string,
  payload?: unknown,
): Promise<Response> {
  return fetch(path, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-csrf-token": await csrfToken(),
    },
    body: payload === undefined ? undefined : JSON.stringify(payload),
  });
}

// 상태를 바꾼 뒤 서버가 낸 오류 문구를 그대로 화면에 보이기 위한 공통 처리.
export async function expectOk(response: Response, fallback: string) {
  if (response.ok) return;

  const body = await response.json().catch(() => null);
  const message = (body as { error?: string } | null)?.error;
  throw new Error(message && message !== "forbidden" ? message : fallback);
}
