"use client";

// 루트 레이아웃이 무너지면 <html>부터 이 파일이 낸다 — 셸도 전역 스타일도 없는
// 자리라 마크업을 여기서 직접 갖춘다.
export default function GlobalError({
  error,
}: {
  error: Error & { digest?: string };
}) {
  return (
    <html lang="ko">
      <body
        style={{
          margin: 0,
          minHeight: "100dvh",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          gap: "0.75rem",
          padding: "0 1.5rem",
          fontFamily: "system-ui, sans-serif",
          color: "#2f3d39",
        }}
      >
        <h1 style={{ margin: 0, fontSize: "27px" }}>위키를 열지 못했습니다</h1>
        <p style={{ margin: 0, fontSize: "12.5px", color: "#6d7d78" }}>
          {error.message || "요청을 처리하지 못했습니다."}
        </p>
      </body>
    </html>
  );
}
