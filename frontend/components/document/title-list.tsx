import Link from "next/link";
import { Notice } from "@/components/layout/notice";
import { wikiPath } from "@/lib/wiki-path";

// 특수 페이지 열 몇 개가 전부 "문서 제목 + 곁말" 한 모양이다(링크된 횟수·바이트·날짜·
// 역링크 종류…). 목록마다 마크업을 새로 짜지 않도록 이 한 벌로 모은다.
export type TitleEntry = {
  title: string;
  note?: string;
};

export function TitleList({
  entries,
  empty,
}: {
  entries: TitleEntry[];
  empty: string;
}) {
  if (entries.length === 0) return <Notice>{empty}</Notice>;

  return (
    <ol className="m-0 list-none p-0">
      {entries.map((entry) => (
        <li
          key={entry.title}
          className="text-list flex items-baseline gap-2.5 border-b border-line-soft py-2"
        >
          <Link
            href={wikiPath.read(entry.title)}
            className="text-link hover:underline"
          >
            {entry.title}
          </Link>
          {entry.note && <span className="text-faint">{entry.note}</span>}
        </li>
      ))}
    </ol>
  );
}
