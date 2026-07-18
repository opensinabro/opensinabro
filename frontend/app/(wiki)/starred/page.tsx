import { TitleListPage } from "@/components/layout/title-list-page";
import { fetchStarred } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("구독한 문서") };

export default async function StarredPage() {
  return (
    <TitleListPage
      title="구독한 문서"
      note="바뀌면 알림을 받는 문서"
      result={await fetchStarred()}
      empty="아직 구독한 문서가 없습니다."
    />
  );
}
