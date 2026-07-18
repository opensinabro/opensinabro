import Link from "next/link";
import { AccountMenu } from "@/components/layout/account-menu";
import { Section } from "@/components/ui/section";
import { wikiPath } from "@/lib/wiki-path";
import type { SessionView } from "@/lib/api/types";

type NavigationItem = {
  href: string;
  label: string;
};

const browse: NavigationItem[] = [
  { href: "/recent-changes", label: "최근 변경" },
  { href: "/recent-discussions", label: "최근 토론" },
  { href: "/edit-requests", label: "편집요청" },
  { href: "/needed-pages", label: "필요한 문서" },
  { href: "/random", label: "임의 문서" },
];

const mine: NavigationItem[] = [
  { href: "/starred", label: "구독한 문서" },
  { href: "/upload", label: "파일 올리기" },
];

function NavigationLink({ item }: { item: NavigationItem }) {
  return (
    <Link
      href={item.href}
      className="text-ui rounded px-1.5 py-1 text-body hover:bg-ground-deep"
    >
      {item.label}
    </Link>
  );
}

export function NavigationColumn({ session }: { session: SessionView }) {
  return (
    <nav className="flex h-full flex-col gap-4 border-r border-line bg-ground-sub px-3 py-3.5">
      <Link
        href={wikiPath.read(session.mainDocument)}
        className="px-1.5 text-brand font-extrabold tracking-tight text-ink"
      >
        {session.wikiName}
      </Link>

      <form action="/search">
        <input
          name="q"
          placeholder="문서 검색"
          aria-label="문서 검색"
          className="w-full rounded border border-line bg-ground px-2.5 py-1 text-xs text-body placeholder:text-faint focus:border-accent focus:outline-none"
        />
      </form>

      <Section label="둘러보기">
        <div className="flex flex-col gap-px">
          {browse.map((item) => (
            <NavigationLink key={item.href} item={item} />
          ))}
        </div>
      </Section>

      <Section label="내 활동">
        <div className="flex flex-col gap-px">
          <Link
            href="/notifications"
            className="text-ui flex items-center justify-between rounded px-1.5 py-1 text-body hover:bg-ground-deep"
          >
            알림
            {session.unread > 0 && (
              <span className="rounded-full bg-accent px-1.5 text-label font-semibold text-white tabular-nums">
                {session.unread}
              </span>
            )}
          </Link>
          {mine.map((item) => (
            <NavigationLink key={item.href} item={item} />
          ))}
        </div>
      </Section>

      {/* 로그인 상태는 내비 맨 아래에 상주한다 — 어느 화면에서든 같은 자리다. */}
      <div className="mt-auto border-t border-line pt-3">
        <AccountMenu session={session} />
      </div>
    </nav>
  );
}
