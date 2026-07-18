import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // 브라우저는 axum(3000)으로 들어오고 프록시가 이리로 넘기므로, 개발 서버가 보는
  // Host는 언제나 자기 주소(3001)와 다르다. 허용하지 않으면 개발 런타임이 멎어
  // 화면이 서버가 그린 상태에서 굳는다 (docs/design/07의 프록시 구성).
  allowedDevOrigins: ["127.0.0.1", "localhost"],
};

export default nextConfig;
