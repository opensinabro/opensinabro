import Link from "next/link";
import { ThreadLine, ThreadList } from "@/components/discussion/thread-list";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchRecentDiscussions } from "@/lib/api/discussion";
import { pageTitle } from "@/lib/site";
import { linkStyle } from "@/components/ui/link";

export const metadata = { title: pageTitle("최근 토론") };

const filters = [
  { value: "", label: "전체" },
  { value: "normal", label: "정상" },
  { value: "pause", label: "중단" },
  { value: "close", label: "닫힘" },
];

export default async function RecentDiscussionsPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string }>;
}) {
  const status = (await searchParams).status ?? "";
  const result = await fetchRecentDiscussions(status);
  const threads = result.kind === "found" ? result.data.threads : [];

  return (
    <WikiPage
      header={<PageHeader title="최근 토론" note="최근에 열린 토론" />}
    >
      <nav className="text-ui mb-3 flex gap-3">
        {filters.map((filter) => {
          const href = filter.value
            ? `/recent-discussions?status=${filter.value}`
            : "/recent-discussions";
          return filter.value === status ? (
            <span
              key={filter.label}
              aria-current="page"
              className="font-semibold text-ink"
            >
              {filter.label}
            </span>
          ) : (
            <Link
              key={filter.label}
              href={href}
              className={linkStyle()}
            >
              {filter.label}
            </Link>
          );
        })}
      </nav>

      {threads.length === 0 ? (
        <Notice>토론이 없습니다.</Notice>
      ) : (
        <ThreadList>
          {threads.map((thread) => (
            <ThreadLine key={thread.id} {...thread} document={thread.title} />
          ))}
        </ThreadList>
      )}
    </WikiPage>
  );
}
