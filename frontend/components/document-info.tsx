import Link from "next/link";
import type { DocumentView } from "@/lib/api";

function formatDate(value: string) {
  return new Date(value).toLocaleDateString("ko-KR", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

function Row({ term, children }: { term: string; children: React.ReactNode }) {
  return (
    <>
      <dt className="text-faint">{term}</dt>
      <dd className="m-0 text-right tabular-nums text-body">{children}</dd>
    </>
  );
}

// 우측 정보 열의 알맹이. 좁은 화면에서는 이 블록이 본문 하단으로 내려간다.
export function DocumentInfo({ document }: { document: DocumentView }) {
  const { revision, backlinkCount, threadCount, title } = document;

  return (
    <div className="flex flex-col gap-3.5 text-[12.5px]">
      <div>
        <h2 className="mb-2 text-[10.5px] font-bold tracking-[0.12em] text-faint uppercase">
          문서 정보
        </h2>
        <dl className="m-0 grid grid-cols-[auto_1fr] gap-x-2.5 gap-y-1">
          {revision && (
            <>
              <Row term="리비전">r{revision.sequence}</Row>
              <Row term="편집">{formatDate(revision.createdAt)}</Row>
              <Row term="편집자">{revision.author}</Row>
              <Row term="크기">{revision.contentBytes.toLocaleString()} B</Row>
            </>
          )}
          <Row term="역링크">{backlinkCount.toLocaleString()}</Row>
        </dl>
      </div>

      <div className="border-t border-line-soft" />

      <div>
        <h2 className="mb-2 text-[10.5px] font-bold tracking-[0.12em] text-faint uppercase">
          토론 {threadCount}
        </h2>
        <Link href={`/discuss/${title}`} className="text-link hover:underline">
          {threadCount > 0 ? "열린 토론 보기" : "토론 열기"}
        </Link>
      </div>

      <div className="border-t border-line-soft" />

      <div>
        <h2 className="mb-2 text-[10.5px] font-bold tracking-[0.12em] text-faint uppercase">
          도구
        </h2>
        <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
          <li>
            <Link
              href={`/backlink/${title}`}
              className="text-link hover:underline"
            >
              역링크 보기
            </Link>
          </li>
          <li>
            <Link href={`/raw/${title}`} className="text-link hover:underline">
              원문 보기
            </Link>
          </li>
          <li>
            <Link href={`/move/${title}`} className="text-link hover:underline">
              문서 이동
            </Link>
          </li>
          <li>
            <Link href={`/acl/${title}`} className="text-link hover:underline">
              ACL 확인
            </Link>
          </li>
        </ul>
      </div>
    </div>
  );
}
