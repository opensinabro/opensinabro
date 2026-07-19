import { Link } from "@/components/layout/link";
import { wikiPath } from "@/lib/wiki-path";

// 문서 동작 탭은 곧 URL이다 — 탭 하나가 라우트 하나에 대응한다 (docs/architecture.md의 URL 설계).
// 역링크·원문처럼 탭에 없는 도구 화면에서도 같은 탭 줄을 내걸어, 어느 문서 화면에 있든
// 돌아가는 길이 같은 자리에 있게 한다.
//
// "읽기"는 내걸지 않는다 — 제목이 이미 읽는 화면을 가리키므로, 그 자리로 돌아가는 길은
// 제목 자체다. 나머지 셋은 탭이 아니라 단추로 보이게 해 누를 것임을 드러낸다.
const tabs = [
  { key: "edit", label: "편집", href: wikiPath.edit },
  { key: "history", label: "역사", href: wikiPath.history },
  { key: "discuss", label: "토론", href: wikiPath.discuss },
] as const;

export type DocumentTab = "read" | (typeof tabs)[number]["key"];

const tabStyle =
  "text-ui rounded border border-line px-3 py-1 text-body hover:border-accent hover:text-accent-deep";
const currentTabStyle =
  "text-ui rounded border border-accent px-3 py-1 font-semibold text-accent-deep";

export function DocumentActions({
  title,
  current,
}: {
  title: string;
  current?: DocumentTab;
}) {
  return (
    <nav className="flex flex-wrap items-center gap-1.5">
      {tabs.map((tab) =>
        tab.key === current ? (
          <span key={tab.key} aria-current="page" className={currentTabStyle}>
            {tab.label}
          </span>
        ) : (
          <Link key={tab.key} href={tab.href(title)} className={tabStyle}>
            {tab.label}
          </Link>
        ),
      )}
    </nav>
  );
}
