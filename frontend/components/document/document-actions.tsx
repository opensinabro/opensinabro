import Link from "next/link";
import { wikiPath } from "@/lib/wiki-path";

// 문서 동작 탭은 곧 URL이다 — 탭 하나가 라우트 하나에 대응한다 (docs/architecture.md의 URL 설계).
// 역링크·원문처럼 탭에 없는 도구 화면에서도 같은 탭 줄을 내걸어, 어느 문서 화면에 있든
// 돌아가는 길이 같은 자리에 있게 한다.
const tabs = [
  { key: "read", label: "읽기", href: wikiPath.read },
  { key: "edit", label: "편집", href: wikiPath.edit },
  { key: "history", label: "역사", href: wikiPath.history },
  { key: "discuss", label: "토론", href: wikiPath.discuss },
] as const;

export type DocumentTab = (typeof tabs)[number]["key"];

export function DocumentActions({
  title,
  current,
}: {
  title: string;
  current?: DocumentTab;
}) {
  return (
    <nav className="flex gap-0.5">
      {tabs.map((tab) =>
        tab.key === current ? (
          <span
            key={tab.key}
            aria-current="page"
            className="text-ui rounded-t border border-b-0 border-line bg-ground px-2.5 py-1 font-semibold text-ink"
          >
            {tab.label}
          </span>
        ) : (
          <Link
            key={tab.key}
            href={tab.href(title)}
            className="text-ui rounded-t border border-b-0 border-transparent px-2.5 py-1 text-muted hover:text-ink"
          >
            {tab.label}
          </Link>
        ),
      )}
    </nav>
  );
}
