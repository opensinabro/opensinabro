"use client";

import NextLink, { useLinkStatus } from "next/link";
import { useEffect } from "react";
import { useNavigationProgress } from "@/lib/navigation-progress";

// 셸의 링크는 전부 이걸 거친다. next/link를 직접 쓰면 그 링크만 이동 중임을 알리지
// 못하는데, 화면은 응답이 올 때까지 이전 상태 그대로라 그 한 곳이 "눌리지 않는
// 링크"가 된다 — 라우트 그룹의 loading.tsx를 걷어낸 대가를 여기서 갚는다.
function NavigationSignal() {
  const { pending } = useLinkStatus();

  useEffect(() => {
    if (!pending) return;

    const { begin, finish } = useNavigationProgress.getState();
    begin();
    return finish;
  }, [pending]);

  // 누른 링크 자신도 흐려진다. 화면 꼭대기의 띠는 눈이 가 있지 않을 수 있어, 손이
  // 머문 자리에서도 신호가 나야 한다. 뜨는 시점은 CSS가 늦춘다.
  return pending ? <span hidden data-navigating /> : null;
}

export function Link({
  children,
  ...props
}: React.ComponentProps<typeof NextLink>) {
  return (
    <NextLink {...props}>
      {children}
      <NavigationSignal />
    </NextLink>
  );
}
