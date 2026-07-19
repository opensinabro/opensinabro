"use client";

import { Link } from "@/components/layout/link";
import { useEffect, useRef } from "react";
import { StarButton } from "@/components/document/star-button";
import { wikiPath } from "@/lib/wiki-path";

// 문서에 딸린 도구는 전부 이 한 칸에 접어 둔다 — 제목 줄 오른쪽은 편집·역사·토론이
// 차지하고, 그보다 덜 쓰는 것들이 그 옆에 줄지어 서면 무엇을 눌러야 할지가 흐려진다.
//
// ACL 편집 화면은 아직 서버에 라우트가 없어 여기 걸지 않는다 — 없는 곳으로 가는
// 링크를 남겨 두면 화면이 멀쩡해 보이면서 죽은 길이 생긴다.
const menuStyle =
  "text-ui rounded px-2 py-1 text-left text-muted hover:bg-ground-deep hover:text-ink";

// 문서 전체가 아니라 이 줄이 쓰는 값만 받는다 — 클라이언트 경계를 넘는 것은 전부
// 직렬화되므로, DocumentView를 통째로 넘기면 렌더 트리까지 HTML에 실려 나간다.
export function DocumentToolbar({
  title,
  starred,
}: {
  title: string;
  starred: boolean;
}) {
  const holder = useRef<HTMLDetailsElement>(null);

  // details는 스스로 닫힐 줄 모른다 — 열어 둔 채로 딴 데를 눌러도 그대로 남아 있어
  // 화면에 붙박인 것처럼 보인다. 팝오버라면 바깥을 누르거나 Esc를 치면 닫혀야 한다.
  useEffect(() => {
    const close = () => {
      if (holder.current) holder.current.open = false;
    };

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (target instanceof Node && holder.current?.contains(target)) return;
      close();
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };

    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);

    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const links = [
    { label: "역링크", href: wikiPath.backlink },
    { label: "원문", href: wikiPath.raw },
    { label: "작성자 보기", href: wikiPath.blame },
    { label: "문서 이동", href: wikiPath.move },
    { label: "문서 삭제", href: wikiPath.delete },
  ];

  return (
    <details ref={holder} className="relative">
      <summary
        aria-label="문서 도구 더 보기"
        className="text-ui list-none rounded border border-line px-3 py-1 text-body hover:border-accent hover:text-accent-deep [&::-webkit-details-marker]:hidden"
      >
        ⋯
      </summary>
      {/* 안에서 무엇을 고르든 할 일은 끝났으므로 함께 닫는다. 링크를 눌러도 클라이언트
          이동이라 이 마크업은 살아남아, 닫아 주지 않으면 다음 화면까지 따라간다. */}
      <div
        onClick={() => {
          if (holder.current) holder.current.open = false;
        }}
        className="absolute top-full right-0 z-20 mt-1 flex w-[140px] flex-col gap-px rounded border border-line bg-ground p-1.5 shadow-[0_10px_28px_-18px_#14201d80]"
      >
        <StarButton title={title} starred={starred} className={menuStyle} />
        {links.map((link) => (
          <Link key={link.label} href={link.href(title)} className={menuStyle}>
            {link.label}
          </Link>
        ))}
      </div>
    </details>
  );
}
