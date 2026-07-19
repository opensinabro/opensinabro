"use client";

import { usePathname } from "next/navigation";
import { ErrorScreen } from "@/components/layout/error-screen";
import { readableAddress } from "@/lib/wiki-path";

// 어느 라우트에도 걸리지 않은 주소. 여기까지 온 요청은 셸을 세울 세션조차 읽은 적이
// 없으므로 헤더·푸터 없이 홀로 선다 — 그래서 갈 곳을 본문이 직접 준다.
//
// 없는 *문서*는 이 화면이 아니라 w/[...title]/not-found.tsx가 받는다. 그쪽은 제목이
// 살아 있어 "지금 만들기"를 줄 수 있지만, 여기는 문서 주소조차 아니라 줄 것이 없다.
export default function NotFound() {
  const address = readableAddress(usePathname());

  return (
    <main className="mx-auto flex min-h-dvh max-w-[520px] flex-col justify-center px-6">
      <ErrorScreen
        title="그런 주소가 없습니다"
        subject={address}
        description="주소가 잘못되었거나, 가리키던 화면이 없어졌습니다."
        actions={[
          { label: "대문으로", href: "/" },
          { label: "검색", href: "/search" },
        ]}
      />
    </main>
  );
}
