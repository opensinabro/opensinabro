"use client";

import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { buttonStyle } from "@/components/ui/button";

// 화면 하나가 실패해도 셸(내비·푸터)은 살아 있어야 다른 문서로 빠져나갈 수 있다.
export default function WikiError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <WikiPage
      header={<PageHeader title="오류" note="이 화면을 그리지 못했습니다" />}
    >
      <p className="text-note m-0 py-2 text-muted">
        {error.message || "요청을 처리하지 못했습니다."}
      </p>
      <div className="mt-4">
        <button type="button" onClick={reset} className={buttonStyle()}>
          다시 시도
        </button>
      </div>
    </WikiPage>
  );
}
