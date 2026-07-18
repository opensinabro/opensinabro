import Link from "next/link";

const links = [
  { href: "/orphaned-pages", label: "고립된 문서" },
  { href: "/uncategorized-pages", label: "분류가 없는 문서" },
  { href: "/old-pages", label: "오래된 문서" },
  { href: "/shortest-pages", label: "내용이 짧은 문서" },
  { href: "/longest-pages", label: "내용이 긴 문서" },
  { href: "/block-history", label: "운영 기록" },
  { href: "/license", label: "라이선스" },
];

// 자주 쓰지 않는 특수 페이지는 내비를 늘리지 않고 여기에 둔다 — 내비는 매일 쓰는
// 길만 담아야 짧게 유지된다.
export function SiteFooter({ contentLicense }: { contentLicense: string }) {
  return (
    <footer className="text-fine mt-auto border-t border-line px-4 py-3.5 text-faint sm:px-6">
      <p className="m-0">문서 내용은 {contentLicense}를 따릅니다.</p>
      <p className="m-0 mt-1.5 flex flex-wrap gap-x-2.5 gap-y-1">
        {links.map((link) => (
          <Link key={link.href} href={link.href} className="hover:text-accent-deep">
            {link.label}
          </Link>
        ))}
      </p>
    </footer>
  );
}
