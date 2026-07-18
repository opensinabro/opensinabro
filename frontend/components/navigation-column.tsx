import Link from "next/link";

type NavigationItem = {
  href: string;
  label: string;
};

const browse: NavigationItem[] = [
  { href: "/recent-changes", label: "최근 변경" },
  { href: "/recent-discussions", label: "최근 토론" },
  { href: "/needed-pages", label: "필요한 문서" },
  { href: "/random", label: "임의 문서" },
];

const mine: NavigationItem[] = [
  { href: "/starred", label: "구독한 문서" },
  { href: "/notifications", label: "알림" },
];

function Group({ label, items }: { label: string; items: NavigationItem[] }) {
  return (
    <div className="flex flex-col gap-px">
      <div className="px-1.5 pb-1.5 text-[10.5px] font-bold tracking-[0.12em] text-faint uppercase">
        {label}
      </div>
      {items.map((item) => (
        <Link
          key={item.href}
          href={item.href}
          className="rounded px-1.5 py-1 text-[13px] text-body hover:bg-ground-deep"
        >
          {item.label}
        </Link>
      ))}
    </div>
  );
}

export function NavigationColumn({ wikiName }: { wikiName: string }) {
  return (
    <nav className="flex h-full flex-col gap-4 border-r border-line bg-ground-sub px-3 py-3.5">
      <Link
        href="/"
        className="px-1.5 text-[14.5px] font-extrabold tracking-tight text-ink"
      >
        {wikiName}
      </Link>

      <form action="/search" className="contents">
        <input
          name="q"
          placeholder="문서 검색"
          aria-label="문서 검색"
          className="rounded border border-line bg-ground px-2.5 py-1 text-xs text-body placeholder:text-faint focus:border-accent focus:outline-none"
        />
      </form>

      <Group label="둘러보기" items={browse} />
      <Group label="내 활동" items={mine} />
    </nav>
  );
}
