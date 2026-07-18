import Link from "next/link";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { BatchRevertButton } from "@/components/operate/batch-revert-button";
import { buttonStyle } from "@/components/ui/button";
import { inputStyle } from "@/components/ui/field";
import { LinkList, Section } from "@/components/ui/section";
import { fetchBatchRevertTargets } from "@/lib/api/operate";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

type BatchRevertPageProps = {
  searchParams: Promise<{ author?: string }>;
};

export const metadata = { title: pageTitle("일괄 되돌리기") };

export default async function BatchRevertPage({
  searchParams,
}: BatchRevertPageProps) {
  const author = (await searchParams).author?.trim() ?? "";
  const result = await fetchBatchRevertTargets(author);

  const header = (
    <PageHeader
      title="일괄 되돌리기"
      note="한 사람이 마지막으로 손댄 문서를 한꺼번에 되돌립니다"
    />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 화면을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  const { titles } = result.data;

  return (
    <WikiPage header={header}>
      {/* 검색은 주소에 남아야 새로 고쳐도 같은 목록이 나온다 — 그래서 GET 폼이다. */}
      <form method="get" className="mt-4 flex items-center gap-2">
        <label htmlFor="batch-revert-author" className="text-note text-muted">
          작성자
        </label>
        <input
          id="batch-revert-author"
          name="author"
          defaultValue={author}
          className={`${inputStyle} max-w-[220px]`}
        />
        <button type="submit" className={buttonStyle()}>
          찾기
        </button>
      </form>

      {author === "" ? (
        <Notice>되돌릴 편집의 작성자를 적으세요.</Notice>
      ) : titles.length === 0 ? (
        <Notice>{author}가 마지막으로 손댄 문서가 없습니다.</Notice>
      ) : (
        <>
          <div className="mt-6">
            <Section label={`되돌릴 문서 ${titles.length}개`}>
              <LinkList>
                {titles.map((title) => (
                  <li key={title} className="text-list">
                    <Link
                      href={wikiPath.read(title)}
                      className="text-link hover:underline"
                    >
                      {title}
                    </Link>
                  </li>
                ))}
              </LinkList>
            </Section>
          </div>
          <BatchRevertButton author={author} />
        </>
      )}
    </WikiPage>
  );
}
