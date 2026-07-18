import { TitleListPage } from "@/components/layout/title-list-page";
import { fetchOldPages } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("오래된 문서") };

export default async function OldPagesPage() {
  return (
    <TitleListPage
      title="오래된 문서"
      note="가장 오래 손대지 않은 문서"
      result={await fetchOldPages()}
      empty="문서가 없습니다."
    />
  );
}
