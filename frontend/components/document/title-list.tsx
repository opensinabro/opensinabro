import { Link } from "@/components/layout/link";
import { Notice } from "@/components/layout/notice";
import { linkStyle } from "@/components/ui/link";
import { Row, Rows } from "@/components/ui/list";
import { wikiPath } from "@/lib/wiki-path";
import type { TitleEntry } from "@/lib/api/types";

// 특수 페이지 열 몇 개가 전부 "문서 제목 + 곁말" 한 모양이다(링크된 횟수·바이트·날짜·
// 역링크 종류…). 목록마다 마크업을 새로 짜지 않도록 이 한 벌로 모은다.
export function TitleList({
  entries,
  empty,
}: {
  entries: TitleEntry[];
  empty: string;
}) {
  if (entries.length === 0) return <Notice>{empty}</Notice>;

  return (
    <Rows as="ol">
      {entries.map((entry) => (
        <Row key={entry.title} shape="compact">
          <Link href={wikiPath.read(entry.title)} className={linkStyle()}>
            {entry.title}
          </Link>
          {entry.note && <span className="text-faint">{entry.note}</span>}
        </Row>
      ))}
    </Rows>
  );
}
