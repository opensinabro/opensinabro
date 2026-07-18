import Link from "next/link";
import { AccountMenu } from "@/components/layout/account-menu";
import { SearchForm } from "@/components/layout/search-form";
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

// 넓은 화면의 좌측 열과 좁은 화면의 서랍이 같은 내용을 쓴다. 목록을 두 벌로 두면
// 한쪽에만 항목이 추가되어 화면 폭에 따라 갈 수 있는 곳이 달라진다.
export function NavigationContent({
  session,
  searchId,
}: {
  session: SessionView;
  searchId?: string;
}) {
  return (
    <>
      <SearchForm id={searchId} />

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
    </>
  );
}

export function WikiNameLink({
  session,
  className,
}: {
  session: SessionView;
  className?: string;
}) {
  return (
    <Link
      href={wikiPath.read(session.mainDocument)}
      className={`text-brand font-extrabold tracking-tight text-ink ${className ?? ""}`}
    >
      {session.wikiName}
    </Link>
  );
}

export function NavigationColumn({ session }: { session: SessionView }) {
  return (
    <nav className="flex h-full flex-col gap-4 border-r border-line bg-ground-sub px-3 py-3.5">
      <WikiNameLink session={session} className="px-1.5" />
      <NavigationContent session={session} searchId="search-column" />
    </nav>
  );
}
