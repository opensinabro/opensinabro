"use client";

// 루트 레이아웃이 무너지면 <html>부터 이 파일이 낸다 — 셸도 전역 스타일도 없는
// 자리라 마크업을 여기서 직접 갖춘다. 색과 크기는 globals.css의 토큰을 손으로 옮긴
// 값이다 (ink #000 · body #121a18 · muted #24302d · faint #36423f, 제목 30 · 본문 14.5).
// 옛 값(#2f3d39·#6d7d78)은 회색조라 "연하게 해서 위계를 나른다"는 금지를 어기고 있었다.
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
          gap: "12px",
          padding: "0 24px",
          fontFamily: "system-ui, sans-serif",
          color: "#121a18",
        }}
      >
        <h1
          style={{
            margin: 0,
            fontSize: "30px",
            fontWeight: 800,
            letterSpacing: "-0.02em",
            color: "#000000",
          }}
        >
          위키를 열지 못했습니다
        </h1>
        <p style={{ margin: 0, fontSize: "14.5px", color: "#24302d" }}>
          {error.message ||
            "요청을 처리하지 못했습니다. 잠시 뒤 다시 시도해 주세요."}
        </p>
        {error.digest && (
          <p style={{ margin: 0, fontSize: "13.5px", color: "#36423f" }}>
            추적 번호{" "}
            <span style={{ fontFamily: "ui-monospace, monospace" }}>
              {error.digest}
            </span>
          </p>
        )}
        {/* 이 화면에서는 리액트가 통째로 무너진 뒤라 next/link가 남아 있지 않다. */}
        <p style={{ margin: 0, fontSize: "14.5px", color: "#24302d" }}>
          할 수 있는 일{" "}
          {/* eslint-disable-next-line @next/next/no-html-link-for-pages */}
          <a href="/" style={{ color: "#1a5fb4" }}>
            대문으로
          </a>
        </p>
      </body>
    </html>
  );
}
