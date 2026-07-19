"use client";

import { ErrorScreen } from "@/components/layout/error-screen";

// (wiki) 레이아웃이 세션을 읽지 못하면 셸 자체를 그릴 수 없다. 레이아웃의 오류는
// 그 바깥 경계가 받으므로 이 파일이 셸 없는 마지막 안내를 맡는다.
//
// 셸이 없으니 헤더의 길도 없다 — 여기서 갈 곳을 주지 않으면 위키 전체가 막힌다.
export default function RootError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <main className="mx-auto flex min-h-dvh max-w-[520px] flex-col justify-center px-6">
      <ErrorScreen
        title="위키를 열지 못했습니다"
        description={
          error.message ||
          "요청을 처리하지 못했습니다. 잠시 뒤 다시 시도해 주세요."
        }
        diagnostics={
          error.digest
            ? [{ label: "추적 번호", value: error.digest }]
            : undefined
        }
        actions={[{ label: "대문으로", href: "/" }]}
        onRetry={reset}
      />
    </main>
  );
}
