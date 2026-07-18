import type { Metadata } from "next";
import { Providers } from "./providers";
import "./globals.css";

export const metadata: Metadata = {
  title: "오픈시나브로",
  description: "나무위키 엔진의 오픈소스 재구현",
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
        <link rel="stylesheet" href="/style.css" />
      </head>
      <body className="min-h-full">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
