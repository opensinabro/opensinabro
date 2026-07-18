import Link from "next/link";
import { StarButton } from "@/components/document/star-button";
import { LinkList, Section } from "@/components/ui/section";
import { formatBytes, formatCount, formatDay } from "@/lib/format";
import { wikiPath } from "@/lib/wiki-path";
import type { DocumentView } from "@/lib/api/types";
import { linkStyle } from "@/components/ui/link";

function Row({ term, children }: { term: string; children: React.ReactNode }) {
  return (
    <>
      <dt className="text-faint">{term}</dt>
      <dd className="m-0 text-right tabular-nums text-body">{children}</dd>
    </>
  );
}

// ACL 편집 화면은 아직 서버에 라우트가 없어 여기 걸지 않는다 — 없는 곳으로 가는
// 링크를 남겨 두면 화면이 멀쩡해 보이면서 죽은 길이 생긴다.
const tools = [
  { label: "역링크 보기", href: wikiPath.backlink },
  { label: "원문 보기", href: wikiPath.raw },
  { label: "작성자 보기", href: wikiPath.blame },
  { label: "문서 이동", href: wikiPath.move },
  { label: "문서 삭제", href: wikiPath.delete },
];

// 우측 정보 열의 알맹이. 좁은 화면에서는 이 블록이 본문 하단으로 내려간다 (WikiPage).
export function DocumentInfo({ document }: { document: DocumentView }) {
  const { revision, backlinkCount, threadCount, title } = document;

  return (
    <div className="text-note flex flex-col gap-3.5">
      <StarButton title={title} starred={document.starred} />

      <Section label="문서 정보">
        <dl className="m-0 grid grid-cols-[auto_1fr] gap-x-2.5 gap-y-1">
          {revision && (
            <>
              <Row term="리비전">r{revision.sequence}</Row>
              <Row term="편집">{formatDay(revision.createdAt)}</Row>
              <Row term="편집자">{revision.author}</Row>
              <Row term="크기">{formatBytes(revision.contentBytes)}</Row>
            </>
          )}
          <Row term="역링크">{formatCount(backlinkCount)}</Row>
        </dl>
      </Section>

      <div className="border-t border-line-soft" />

      <Section label={`토론 ${threadCount}`}>
        <Link href={wikiPath.discuss(title)} className={linkStyle()}>
          {threadCount > 0 ? "열린 토론 보기" : "토론 열기"}
        </Link>
      </Section>

      <div className="border-t border-line-soft" />

      <Section label="도구">
        <LinkList>
          {tools.map((tool) => (
            <li key={tool.label}>
              <Link
                href={tool.href(title)}
                className={linkStyle()}
              >
                {tool.label}
              </Link>
            </li>
          ))}
        </LinkList>
      </Section>
    </div>
  );
}
