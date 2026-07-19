import { Link } from "@/components/layout/link";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchContributions } from "@/lib/api/account";
import { formatMoment } from "@/lib/format";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";

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
              className={linkStyle({ size: "ui" })}
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
        <Rows>
          {entries.map((entry, index) => (
            <Row key={`${entry.title}:${entry.sequence}:${index}`}>
              <Link
                href={wikiPath.read(entry.title)}
                className={linkStyle({ weight: "medium" })}
              >
                {entry.title}
              </Link>
              <span className="text-muted">r{entry.sequence}</span>
              <span className="text-faint">{formatMoment(entry.createdAt)}</span>
              {entry.comment && (
                <span className="text-body">{entry.comment}</span>
              )}
            </Row>
          ))}
        </Rows>
      )}
    </WikiPage>
  );
}
