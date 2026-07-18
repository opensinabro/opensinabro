"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { postJson } from "@/lib/api/csrf";
import { encodeTitle } from "@/lib/wiki-path";

// 가려진 리비전은 목록에 남고 내용만 감춘다. 되돌리는 길이 같은 자리에 있어야 하므로
// 숨기기와 도로 보이기를 한 단추의 두 상태로 둔다.
export function HideRevisionButton({
  title,
  revisionId,
  hidden,
}: {
  title: string;
  revisionId: string;
  hidden: boolean;
}) {
  const router = useRouter();
  const [working, setWorking] = useState(false);

  return (
    <button
      type="button"
      disabled={working}
      onClick={async () => {
        setWorking(true);
        await postJson(`/api/hide-revision/${encodeTitle(title)}`, {
          uuid: revisionId,
          hidden: !hidden,
        });
        router.refresh();
        setWorking(false);
      }}
      className="text-link hover:underline disabled:opacity-60"
    >
      {hidden ? "도로 보이기" : "가리기"}
    </button>
  );
}
