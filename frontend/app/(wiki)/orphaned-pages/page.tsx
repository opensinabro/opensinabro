import { TitleListPage } from "@/components/layout/title-list-page";
import { fetchOrphanedPages } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("고립된 문서") };

export default async function OrphanedPagesPage() {
  return (
    <TitleListPage
      title="고립된 문서"
      note="어느 문서도 링크하지 않는 문서"
      result={await fetchOrphanedPages()}
      empty="고립된 문서가 없습니다."
    />
  );
}
