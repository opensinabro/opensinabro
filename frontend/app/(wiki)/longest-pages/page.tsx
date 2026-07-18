import { TitleListPage } from "@/components/layout/title-list-page";
import { fetchPagesByLength } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("내용이 긴 문서") };

export default async function LongestPagesPage() {
  return (
    <TitleListPage
      title="내용이 긴 문서"
      result={await fetchPagesByLength("longest")}
      empty="문서가 없습니다."
    />
  );
}
