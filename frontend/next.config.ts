import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // 브라우저는 axum(3000)으로 들어오고 프록시가 이리로 넘기므로, 개발 서버가 보는
  // Host는 언제나 자기 주소(3001)와 다르다. 허용하지 않으면 개발 런타임이 멎어
  // 화면이 서버가 그린 상태에서 굳는다 (docs/architecture.md의 프록시 구성).
  allowedDevOrigins: ["127.0.0.1", "localhost"],

  experimental: {
    // 개발 서버가 문서 한 장에 수 초를 쓰던 원인. 이 채널이 켜져 있으면 RSC 응답을
    // 되읽는 단계에서 참조가 풀릴 때마다 자식의 디버그 정보 배열을 부모로 복사해
    // 누적하고(transferReferencedDebugInfo), 그 비용이 문서가 클수록 초선형으로 는다.
    // 문서 렌더가 표·각주로 노드를 많이 만드는 이 위키에서 특히 크게 나타난다.
    // 끄면 브라우저 React DevTools의 서버 컴포넌트 정보만 사라지고, 오류 오버레이와
    // 스택·코드 프레임·HMR은 그대로다.
    reactDebugChannel: false,
  },
};

export default nextConfig;
