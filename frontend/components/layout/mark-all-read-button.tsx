"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { buttonStyle } from "@/components/ui/button";
import { postJson } from "@/lib/api/csrf";

export function MarkAllReadButton() {
  const router = useRouter();
  const [working, setWorking] = useState(false);

  return (
    <button
      type="button"
      disabled={working}
      onClick={async () => {
        setWorking(true);
        await postJson("/api/notifications/read");
        // 셸의 안 읽은 수도 서버가 그리므로 목록만 새로 받으면 배지가 남는다.
        router.refresh();
        setWorking(false);
      }}
      className={buttonStyle()}
    >
      모두 읽음으로
    </button>
  );
}
