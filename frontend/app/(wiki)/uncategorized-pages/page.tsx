import { TitleListPage } from "@/components/layout/title-list-page";
import { fetchUncategorizedPages } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("분류가 없는 문서") };

export default async function UncategorizedPagesPage() {
  return (
    <TitleListPage
      title="분류가 없는 문서"
      note="어느 분류에도 들어 있지 않은 문서"
      result={await fetchUncategorizedPages()}
      empty="분류가 없는 문서가 없습니다."
    />
  );
}
