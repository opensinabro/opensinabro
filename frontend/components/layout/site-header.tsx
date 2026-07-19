import { Link } from "@/components/layout/link";
import { AccountMenu } from "@/components/layout/account-menu";
import { Hold } from "@/components/layout/hold";
import { LogoMark } from "@/components/layout/logo-mark";
import { SearchForm } from "@/components/layout/search-form";
import { wikiPath } from "@/lib/wiki-path";
import type { SessionView } from "@/lib/api/types";

type NavigationItem = {
  href: string;
  label: string;
};

// 한 줄 헤더는 자리가 한정되어 있으므로 무엇을 드러낼지가 곧 편집 의도다. 앞의 셋만
// 상시로 서고 나머지는 "더보기" 뒤에 둔다 — 드러난 셋이 이 위키에서 매일 가는 곳이다.
const primary: NavigationItem[] = [
  { href: "/recent-changes", label: "최근 변경" },
  { href: "/recent-discussions", label: "최근 토론" },
  { href: "/edit-requests", label: "편집요청" },
];

const secondary: NavigationItem[] = [
  { href: "/needed-pages", label: "필요한 문서" },
  { href: "/random", label: "임의 문서" },
  { href: "/starred", label: "구독한 문서" },
  { href: "/upload", label: "파일 올리기" },
];

function NavigationLink({
  item,
  className,
}: {
  item: NavigationItem;
  className?: string;
}) {
  return (
    <Link
      href={item.href}
      className={`text-ui rounded px-1.5 py-1 text-muted hover:bg-ground-deep hover:text-ink ${className ?? ""}`}
    >
      {item.label}
    </Link>
  );
}

function WikiNameLink({
  session,
  className,
}: {
  session: SessionView;
  className?: string;
}) {
  return (
    <Link
      href={wikiPath.read(session.mainDocument)}
      className={`text-brand flex shrink-0 items-center gap-1.5 font-extrabold tracking-tight text-ink ${className ?? ""}`}
    >
      <LogoMark className="size-8 shrink-0 text-accent" />
      {session.wikiName}
    </Link>
  );
}

// 셸의 크롬은 이 한 줄이 전부다. 좌측 열도 서랍도 없으므로 화면 폭이 달라져도
// 갈 수 있는 곳이 사라지지 않는다 — 좁아지면 링크가 "더보기" 안으로 들어갈 뿐이다.
//
// 여닫이는 <details>로 짠다. 상태를 리액트가 들면 헤더 전체가 클라이언트 컴포넌트가
// 되고, 세션을 읽는 서버 렌더의 이점이 사라진다.
export function SiteHeader({ session }: { session: SessionView }) {
  return (
    <div className="bg-ground">
      <Hold className="flex items-center gap-2 py-2 sm:gap-3">
        <WikiNameLink session={session} />

        <nav className="ml-1 hidden items-center gap-0.5 md:flex">
          {primary.map((item) => (
            <NavigationLink key={item.href} item={item} />
          ))}
        </nav>

        <details className="relative">
          <summary className="text-ui list-none rounded px-1.5 py-1 text-faint hover:bg-ground-deep hover:text-ink [&::-webkit-details-marker]:hidden">
            <span className="hidden md:inline">더보기</span>
            <span className="md:hidden">메뉴</span>
          </summary>
          <div className="absolute top-full left-0 z-20 mt-1 flex w-[168px] flex-col gap-px rounded border border-line bg-ground p-1.5 shadow-[0_10px_28px_-18px_#14201d80]">
            {/* 좁은 화면에서는 상시 링크도 여기 들어온다 — 폭에 따라 갈 수 있는 곳이
                달라지면 안 된다. */}
            <div className="flex flex-col gap-px md:hidden">
              {primary.map((item) => (
                <NavigationLink key={item.href} item={item} />
              ))}
              <span className="my-1 border-t border-line-soft" />
            </div>
            {secondary.map((item) => (
              <NavigationLink key={item.href} item={item} />
            ))}
          </div>
        </details>

        <div className="ml-auto flex items-center gap-2 sm:gap-3">
          <div className="hidden w-[168px] sm:block lg:w-[210px]">
            <SearchForm />
          </div>
          <Link
            href="/search"
            aria-label="문서 검색"
            className="text-ui rounded px-1.5 py-1 text-muted hover:bg-ground-deep hover:text-ink sm:hidden"
          >
            검색
          </Link>

          <Link
            href="/notifications"
            className="text-ui flex shrink-0 items-center gap-1 rounded px-1.5 py-1 text-muted hover:bg-ground-deep hover:text-ink"
          >
            알림
            {session.unread > 0 && (
              <span className="text-label rounded-full bg-accent px-1.5 font-semibold text-white tabular-nums">
                {session.unread}
              </span>
            )}
          </Link>

          <AccountMenu session={session} />
        </div>
      </Hold>
    </div>
  );
}
