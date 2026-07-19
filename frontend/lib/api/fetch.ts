import { headers } from "next/headers";
import { humanMessage } from "@/lib/api/messages";

// 서버 컴포넌트는 프록시를 거치지 않고 axum을 직접 부른다 — 브라우저 → axum →
// Next → axum 왕복을 한 번 줄인다 (docs/architecture.md).
const internalBase =
  process.env.OPENSINABRO_INTERNAL_API ?? "http://127.0.0.1:3000";

// 모든 조회가 같은 네 갈래로 끝난다. 화면은 이 넷만 분기하면 되고, 그 밖의 실패는
// 예외로 올려 Next의 오류 경계가 받는다.
export type Fetched<T> =
  | { kind: "found"; data: T }
  | { kind: "missing" }
  | { kind: "forbidden" }
  | { kind: "unauthorized" };

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

export async function get<T>(path: string): Promise<Fetched<T>> {
  const response = await fetch(`${internalBase}${path}`, {
    headers: await forwardedHeaders(),
    cache: "no-store",
  });

  if (response.status === 404) return { kind: "missing" };
  if (response.status === 403) return { kind: "forbidden" };
  if (response.status === 401) return { kind: "unauthorized" };
  // 남은 것은 오류 경계가 받을 실패다. 서버가 어느 계층에서 넘어졌는지 토큰으로
  // 알려 오므로 그 문장을 그대로 올린다 — 상태 코드만 올리면 화면에 숫자만 남는다.
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as {
      error?: string;
    } | null;

    throw new Error(
      humanMessage(body?.error, `불러오지 못했습니다 (${response.status})`),
    );
  }

  return { kind: "found", data: (await response.json()) as T };
}

// 목록 화면은 실패해도 그릴 것이 있다 — 안내 문구를 보이면 되므로 예외로 올리지 않고
// 갈래를 그대로 넘긴다. 이 헬퍼는 그 갈래를 화면이 쓰기 좋은 모양으로 좁힌다.
export function entriesOf<T>(result: Fetched<{ entries: T[] }>): T[] {
  return result.kind === "found" ? result.data.entries : [];
}
