"use client";

// 상태를 바꾸는 API는 CSRF 토큰을 헤더로 싣는다. 쿠키는 HttpOnly라 스크립트가 읽지
// 못하므로 토큰은 서버가 본문으로 따로 내준다 (docs/architecture.md).
//
// 실패한 약속을 캐시에 남기면 그 탭에서는 새로고침 전까지 어떤 저장도 되지 않는다.
// 잠깐의 네트워크 끊김이 영구 장애가 되지 않도록 실패는 캐시에서 지운다.
let cachedToken: Promise<string> | null = null;

export function csrfToken() {
  cachedToken ??= fetch("/api/csrf", { cache: "no-store" })
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(`보안 토큰을 받지 못했습니다 (${response.status})`);
      }
      const body = (await response.json()) as { token: string };
      return body.token;
    })
    .catch((error: unknown) => {
      cachedToken = null;
      throw error;
    });

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

// 서버는 권한·존재 여부를 기계가 읽을 토큰으로 낸다. 그대로 보이면 한국어 화면에
// 영단어가 튀어나오므로 사람이 읽을 문구로 옮긴다.
const machineMessages: Record<string, string> = {
  forbidden: "권한이 없습니다.",
  unauthorized: "로그인이 필요합니다.",
  not_found: "대상을 찾지 못했습니다.",
  invalid_credentials: "계정 또는 비밀번호가 올바르지 않습니다.",
  internal: "요청을 처리하지 못했습니다.",
};

// 상태를 바꾼 뒤 서버가 낸 오류 문구를 그대로 화면에 보이기 위한 공통 처리.
export async function expectOk(response: Response, fallback: string) {
  if (response.ok) return;

  const body = await response.json().catch(() => null);
  const message = (body as { error?: string } | null)?.error;

  if (!message) throw new Error(fallback);
  throw new Error(machineMessages[message] ?? message);
}
