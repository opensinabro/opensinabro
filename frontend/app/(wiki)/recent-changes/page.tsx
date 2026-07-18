import Link from "next/link";
import {
  RevisionAction,
  RevisionLine,
  RevisionList,
} from "@/components/document/revision-line";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchRecentChanges } from "@/lib/api/server";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";

export const metadata = { title: pageTitle("최근 변경") };

export default async function RecentChangesPage() {
  const result = await fetchRecentChanges();
  const changes = result.kind === "found" ? result.data : [];

  return (
    <WikiPage
      header={<PageHeader title="최근 변경" note="최근에 편집된 문서" />}
    >
      {changes.length === 0 ? (
        <Notice>아직 변경 기록이 없습니다.</Notice>
      ) : (
        <RevisionList>
          {changes.map((change) => (
            <RevisionLine
              key={change.revision.id}
              revision={change.revision}
              lead={
                <Link
                  href={wikiPath.read(change.title)}
                  className={linkStyle({ weight: "medium" })}
                >
                  {change.title}
                </Link>
              }
              actions={
                <RevisionAction
                  href={wikiPath.diff(change.title, change.revision.id)}
                >
                  비교
                </RevisionAction>
              }
            />
          ))}
        </RevisionList>
      )}
    </WikiPage>
  );
}
