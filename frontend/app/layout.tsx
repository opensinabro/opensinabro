import type { Metadata, Viewport } from "next";
import { siteName } from "@/lib/site";
import { Providers } from "./providers";
import "./globals.css";

export const metadata: Metadata = {
  title: siteName,
  description: "나무위키 엔진의 오픈소스 재구현",
};

// 자판이 올라올 때 화면을 밀어 올리지 않고 뷰포트 자체를 줄인다. 편집기의 문법 줄이
// `h-dvh` 안의 맨 아래 칸이라, 이 설정이 없으면 자판 뒤로 숨는다.
export const viewport: Viewport = {
  interactiveWidget: "resizes-content",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ko" className="h-full">
      <head>
        {/* 본문 어휘의 스타일은 렌더러가 소유하고 axum이 같은 오리진에서 내준다.
            번들에 들이면 셸이 소유하게 되어 그 경계가 무너진다. */}
        {/* eslint-disable-next-line @next/next/no-css-tags */}
      </head>
      <body className="min-h-full">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
