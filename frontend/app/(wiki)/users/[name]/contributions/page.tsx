import Link from "next/link";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchContributions } from "@/lib/api/account";
import { formatMoment } from "@/lib/format";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

type PageProps = {
  params: Promise<{ name: string }>;
};

export async function generateMetadata({ params }: PageProps) {
  const { name } = await params;
  return { title: pageTitle(decodeURIComponent(name), "기여 목록") };
}

export default async function ContributionsPage({ params }: PageProps) {
  const name = decodeURIComponent((await params).name);
  const result = await fetchContributions(name);
  const entries = result.kind === "found" ? result.data.entries : [];

  return (
    <WikiPage
      header={
        <PageHeader
          title={name}
          note="기여 목록"
          actions={
            <Link
              href={wikiPath.read(`사용자:${name}`)}
              className="text-ui text-link hover:underline"
            >
              사용자 문서
            </Link>
          }
        />
      }
    >
      {entries.length === 0 ? (
        <Notice>기여가 없습니다.</Notice>
      ) : (
        <ul className="m-0 list-none p-0">
          {entries.map((entry, index) => (
            <li
              key={`${entry.title}:${entry.sequence}:${index}`}
              className="text-list flex flex-wrap items-baseline gap-x-2.5 gap-y-1 border-b border-line-soft py-2.5"
            >
              <Link
                href={wikiPath.read(entry.title)}
                className="font-medium text-link hover:underline"
              >
                {entry.title}
              </Link>
              <span className="text-muted">r{entry.sequence}</span>
              <span className="text-faint">{formatMoment(entry.createdAt)}</span>
              {entry.comment && (
                <span className="text-body">{entry.comment}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </WikiPage>
  );
}
