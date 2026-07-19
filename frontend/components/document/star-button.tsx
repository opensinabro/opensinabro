"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { buttonStyle } from "@/components/ui/button";
import { expectOk, postJson } from "@/lib/api/csrf";
import { encodeTitle } from "@/lib/wiki-path";

// 구독은 로그인한 사람만 할 수 있다. 서버가 401을 내면 로그인으로 보낸다 —
// 단추를 숨기지 않는 이유는 비로그인 사용자에게도 이 기능의 존재가 보여야 해서다.
export function StarButton({
  title,
  starred,
  className,
}: {
  title: string;
  starred: boolean;
  /** 단추 줄이 아니라 메뉴 안에 설 때처럼, 주변에 맞춰 모양을 바꿔야 할 때. */
  className?: string;
}) {
  const router = useRouter();
  const [working, setWorking] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  return (
    <>
      <button
        type="button"
        disabled={working}
        onClick={() => {
          setWorking(true);
          setProblem(null);

          void (async () => {
            try {
              const response = await postJson(`/api/star/${encodeTitle(title)}`);

              if (response.status === 401) {
                router.push("/login");
                return;
              }

              await expectOk(response, "구독 상태를 바꾸지 못했습니다.");
              router.refresh();
            } catch (error) {
              setProblem(
                error instanceof Error
                  ? error.message
                  : "구독 상태를 바꾸지 못했습니다.",
              );
            } finally {
              setWorking(false);
            }
          })();
        }}
        className={
          className ?? buttonStyle({ tone: starred ? "primary" : "quiet" })
        }
      >
        {starred ? "구독 중" : "구독"}
      </button>
      {problem && (
        <span role="alert" className="text-fine text-danger-ink">
          {problem}
        </span>
      )}
    </>
  );
}
