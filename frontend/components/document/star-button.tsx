"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { buttonStyle } from "@/components/ui/button";
import { postJson } from "@/lib/api/csrf";
import { encodeTitle } from "@/lib/wiki-path";

// 구독은 로그인한 사람만 할 수 있다. 서버가 401을 내면 로그인으로 보낸다 —
// 단추를 숨기지 않는 이유는 비로그인 사용자에게도 이 기능의 존재가 보여야 해서다.
export function StarButton({
  title,
  starred,
}: {
  title: string;
  starred: boolean;
}) {
  const router = useRouter();
  const [working, setWorking] = useState(false);

  return (
    <button
      type="button"
      disabled={working}
      onClick={async () => {
        setWorking(true);
        const response = await postJson(`/api/star/${encodeTitle(title)}`);
        setWorking(false);

        if (response.status === 401) {
          router.push("/login");
          return;
        }
        router.refresh();
      }}
      className={buttonStyle({ tone: starred ? "primary" : "quiet" })}
    >
      {starred ? "구독 중" : "구독"}
    </button>
  );
}
