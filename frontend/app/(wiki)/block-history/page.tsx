import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { Section } from "@/components/ui/section";
import { fetchBlockHistory } from "@/lib/api/account";
import { formatMoment } from "@/lib/format";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("운영 기록") };

const entryStyle =
  "text-list flex flex-wrap items-baseline gap-x-2.5 gap-y-1 border-b border-line-soft py-2.5";

export default async function BlockHistoryPage() {
  const result = await fetchBlockHistory();

  if (result.kind !== "found") {
    return (
      <WikiPage header={<PageHeader title="운영 기록" />}>
        <Notice>운영 기록을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const { blocks, permissions } = result.data;

  return (
    <WikiPage
      header={
        <PageHeader title="운영 기록" note="차단과 권한 부여 이력" />
      }
    >
      <div className="flex flex-col gap-7">
        <Section label="차단 기록">
          {blocks.length === 0 ? (
            <Notice>차단 기록이 없습니다.</Notice>
          ) : (
            <ul className="m-0 list-none p-0">
              {blocks.map((block, index) => (
                <li
                  key={`${block.target}:${block.createdAt}:${index}`}
                  className={entryStyle}
                >
                  <span className="font-medium text-ink">{block.target}</span>
                  <span className="text-faint">→</span>
                  <span className="text-body">{block.group}</span>
                  <span className="text-muted">{block.reason}</span>
                  <span className="ml-auto text-faint">
                    {formatMoment(block.createdAt)}
                    {block.removedAt &&
                      ` · 해제 ${formatMoment(block.removedAt)}`}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Section>

        <Section label="권한 기록">
          {permissions.length === 0 ? (
            <Notice>권한 기록이 없습니다.</Notice>
          ) : (
            <ul className="m-0 list-none p-0">
              {permissions.map((entry, index) => (
                <li
                  key={`${entry.userName}:${entry.grantedAt}:${index}`}
                  className={entryStyle}
                >
                  <span className="font-medium text-ink">{entry.userName}</span>
                  <span className="text-body">{entry.permission}</span>
                  <span className="ml-auto text-faint">
                    부여 {formatMoment(entry.grantedAt)}
                    {entry.revokedAt &&
                      ` · 회수 ${formatMoment(entry.revokedAt)}`}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Section>
      </div>
    </WikiPage>
  );
}
