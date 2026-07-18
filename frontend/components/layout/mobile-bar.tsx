"use client";

import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";

// 좌측 내비는 lg 미만에서 걷힌다. 대신 이 상단 바가 같은 내용을 서랍으로 연다 —
// 걷기만 하면 그 폭에서는 검색·로그인·알림으로 가는 길이 통째로 사라진다.
//
// 내비 내용은 서버가 그려 children으로 받는다. 여는 상태만 이쪽이 가진다.
export function MobileBar({
  brand,
  children,
}: {
  brand: React.ReactNode;
  children: React.ReactNode;
}) {
  // 서랍은 "연 화면"으로 기억한다 — 그러면 서랍 안의 링크로 이동했을 때 경로가
  // 달라지면서 저절로 닫힌다. 닫는 일을 이동 이벤트마다 따로 챙길 필요가 없다.
  const pathname = usePathname();
  const [openedAt, setOpenedAt] = useState<string | null>(null);
  const open = openedAt === pathname;
  const closeButton = useRef<HTMLButtonElement>(null);

  const setOpen = (next: boolean) => setOpenedAt(next ? pathname : null);

  useEffect(() => {
    if (!open) return;

    closeButton.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenedAt(null);
    };

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open]);

  return (
    <div className="lg:hidden">
      <div className="flex items-center justify-between gap-3 border-b border-line bg-ground-sub px-4 py-2.5">
        {brand}
        <button
          type="button"
          aria-expanded={open}
          aria-controls="navigation-drawer"
          onClick={() => setOpen(!open)}
          className="text-ui rounded border border-line px-2.5 py-1 text-body focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
        >
          메뉴
        </button>
      </div>

      {open && (
        <>
          <button
            type="button"
            aria-label="메뉴 닫기"
            onClick={() => setOpen(false)}
            className="fixed inset-0 z-10 bg-ink/25"
          />
          <nav
            id="navigation-drawer"
            className="fixed inset-y-0 right-0 z-20 flex w-[264px] max-w-[80vw] flex-col gap-4 overflow-y-auto border-l border-line bg-ground-sub px-3 py-3.5"
          >
            <button
              ref={closeButton}
              type="button"
              onClick={() => setOpen(false)}
              className="text-ui self-end rounded px-1.5 py-1 text-muted hover:bg-ground-deep focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
            >
              닫기
            </button>
            {children}
          </nav>
        </>
      )}
    </div>
  );
}
