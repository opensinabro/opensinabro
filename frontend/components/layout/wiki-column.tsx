import { Link } from "@/components/layout/link";
import { LinkList, Section } from "@/components/ui/section";
import { linkStyle } from "@/components/ui/link";
import { fetchRecentDiscussions } from "@/lib/api/discussion";
import { fetchRecentChanges } from "@/lib/api/server";
import { formatMoment, formatRelative, revisionKindLabel } from "@/lib/format";
import { wikiPath } from "@/lib/wiki-path";

// 우측 열은 "이 문서"가 아니라 "이 위키"를 싣는다. 문서에 딸린 것(리비전·도구)은 제목
// 아래 줄로 올라갔고, 그 자리를 위키가 지금 어떻게 움직이는지가 대신한다 — 그래서
// 이 열은 페이지가 아니라 레이아웃 쪽 어휘이고, 문서가 아닌 화면에서도 같은 자리에 선다.
//
// 한 줄 헤더가 둘러보기를 "더보기" 뒤로 접었기 때문에 갈 곳이 화면에서 사라졌다.
// 이 열이 그것을 갚는다.
// 이 열은 각 묶음의 앞 몇 줄만 싣는다. 나머지가 어디 있는지 말해 주지 않으면 잘린 목록은
// 그것이 전부인 것처럼 보인다.
function ColumnMore({ href }: { href: string }) {
  return (
    <Link href={href} className={`${linkStyle()} text-label`}>
      전체
    </Link>
  );
}

export async function WikiColumn() {
  const [changes, discussions] = await Promise.all([
    fetchRecentChanges(),
    fetchRecentDiscussions("normal"),
  ]);

  const recent = changes.kind === "found" ? changes.data.slice(0, 5) : [];
  const threads =
    discussions.kind === "found" ? discussions.data.threads.slice(0, 4) : [];

  // 둘 다 비어 있으면 열 자체를 그리지 않는다 — 제목만 남은 빈 열은 위키가 죽어
  // 있다는 인상만 준다.
  if (recent.length === 0 && threads.length === 0) return null;

  return (
    // 옆에 설 때는 위아래로 쌓지만, 아래로 내려오면 본문 폭을 그대로 물려받는다 —
    // 그 폭에 한 줄짜리 목록을 세로로만 쌓으면 오른쪽 절반이 빈다. 두 묶음을 나란히
    // 놓아 아래에 붙는 덩어리의 높이를 줄인다.
    <aside className="text-note mt-8 grid grid-cols-1 gap-x-7 gap-y-4 border-t border-line pt-4 sm:max-column:grid-cols-2 column:mt-0 column:border-t-0 column:pt-0">
      {recent.length > 0 && (
        <Section
          label="최근 바뀐 문서"
          size="title"
          action={<ColumnMore href="/recent-changes" />}
        >
          <LinkList>
            {recent.map((change) => (
              <li
                key={change.revision.id}
                className="flex items-baseline justify-between gap-2"
              >
                <Link
                  href={wikiPath.read(change.title)}
                  className={`${linkStyle()} truncate`}
                >
                  {change.title}
                </Link>
                {/* 상대 표기는 눈금이 굵어 정확한 시각을 잃는다 — 정확히 알아야 할 때를
                    위해 절대 시각을 툴팁으로 남긴다. */}
                <span
                  className="shrink-0 text-faint"
                  title={formatMoment(change.revision.createdAt)}
                >
                  {change.revision.kind === "create" ? (
                    <span className="text-accent-deep">
                      {revisionKindLabel(change.revision.kind)}
                    </span>
                  ) : (
                    formatRelative(change.revision.createdAt)
                  )}
                </span>
              </li>
            ))}
          </LinkList>
        </Section>
      )}

      {threads.length > 0 && (
        <>
          {/* 두 묶음이 세로로 이어질 때만 사이를 가른다. 나란히 설 때는 가를 것이
              위아래에 없다. */}
          <div className="border-t border-line-soft sm:max-column:hidden" />
          <Section
            label="열린 토론"
            size="title"
            action={<ColumnMore href="/recent-discussions" />}
          >
            <LinkList>
              {threads.map((thread) => (
                <li key={thread.id} className="flex flex-col">
                  <Link
                    href={wikiPath.discussThread(thread.id)}
                    className={`${linkStyle()} truncate`}
                  >
                    {thread.topic}
                  </Link>
                  <span className="truncate text-faint">{thread.title}</span>
                </li>
              ))}
            </LinkList>
          </Section>
        </>
      )}
    </aside>
  );
}
