"use client";

import { useNavigationProgress } from "@/lib/navigation-progress";

// 화면 꼭대기의 실선 하나. 자리를 차지하지 않으므로 이동이 끝나고 새 화면이 들어와도
// 글이 밀리지 않는다 — 본문 자리를 비우는 로딩 화면과 다른 점이 이것이다.
export function NavigationProgress() {
  const navigating = useNavigationProgress((state) => state.waiting > 0);

  return (
    <div
      aria-hidden
      className="nav-progress"
      data-navigating={navigating || undefined}
    />
  );
}
