"use client";

import { ErrorScreen } from "@/components/layout/error-screen";
import { Hold } from "@/components/layout/hold";

// 화면 하나가 실패해도 셸(헤더·푸터)은 살아 있어야 다른 문서로 빠져나갈 수 있다.
//
// 오류 경계는 클라이언트 컴포넌트여야 하므로 WikiPage를 쓸 수 없다 — 그쪽은 위키 열을
// 서버에서 받아 오기 때문이다. 어차피 실패한 화면 옆에 "최근 바뀐 문서"를 놓을 이유도
// 없으므로, 여기서는 본문 칸만 직접 세운다.
export default function WikiError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <Hold className="flex flex-1 flex-col pt-5 pb-7">
      <main id="content" className="flex min-w-0 flex-col">
        <ErrorScreen
          title="화면을 그리지 못했습니다"
          description={
            error.message ||
            "요청을 처리하지 못했습니다. 잠시 뒤 다시 시도해 주세요."
          }
          // 배포판에서는 Next가 오류 내용을 지우고 digest만 남긴다. 그 번호가 화면과
          // 서버 기록을 잇는 유일한 끈이므로, 있으면 옮겨 적을 수 있게 보인다.
          diagnostics={
            error.digest
              ? [{ label: "추적 번호", value: error.digest }]
              : undefined
          }
          actions={[
            { label: "대문으로", href: "/" },
            { label: "최근 변경", href: "/recent-changes" },
          ]}
          onRetry={reset}
        />
      </main>
    </Hold>
  );
}
