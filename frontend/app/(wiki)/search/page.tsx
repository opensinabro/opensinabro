import { redirect } from "next/navigation";
import { TitleList } from "@/components/document/title-list";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchSearch } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

type PageProps = {
  searchParams: Promise<{ q?: string }>;
};

export async function generateMetadata({ searchParams }: PageProps) {
  const query = (await searchParams).q?.trim() ?? "";
  return { title: pageTitle(query || "검색", query ? "검색" : undefined) };
}

export default async function SearchPage({ searchParams }: PageProps) {
  const query = (await searchParams).q?.trim() ?? "";

  if (!query) {
    return (
      <WikiPage header={<PageHeader title="검색" />}>
        <Notice>검색어를 입력하세요.</Notice>
      </WikiPage>
    );
  }

  const result = await fetchSearch(query);

  // 제목이 정확히 맞으면 목록을 거치지 않고 그 문서로 보낸다 (the seed의 "이동" 동작).
  if (result.kind === "found" && result.data.redirect) {
    redirect(wikiPath.read(result.data.redirect));
  }

  const results = result.kind === "found" ? result.data.results : [];

  return (
    <WikiPage
      header={<PageHeader title={query} note="검색 결과" />}
    >
      <TitleList
        entries={results}
        empty="찾는 문서가 없습니다. 제목으로 새로 만들 수 있습니다."
      />
    </WikiPage>
  );
}
