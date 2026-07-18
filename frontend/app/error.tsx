"use client";

import { buttonStyle } from "@/components/ui/button";

// (wiki) 레이아웃이 세션을 읽지 못하면 셸 자체를 그릴 수 없다. 레이아웃의 오류는
// 그 바깥 경계가 받으므로 이 파일이 셸 없는 마지막 안내를 맡는다.
export default function RootError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div className="mx-auto flex min-h-dvh max-w-[520px] flex-col justify-center gap-4 px-6">
      <h1 className="text-title m-0 font-extrabold tracking-tight text-ink">
        위키를 열지 못했습니다
      </h1>
      <p className="text-note m-0 text-muted">
        {error.message || "요청을 처리하지 못했습니다."}
      </p>
      <div>
        <button type="button" onClick={reset} className={buttonStyle()}>
          다시 시도
        </button>
      </div>
    </div>
  );
}
