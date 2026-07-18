import { TitleListPage } from "@/components/layout/title-list-page";
import { fetchNeededPages } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("필요한 문서") };

export default async function NeededPagesPage() {
  return (
    <TitleListPage
      title="필요한 문서"
      note="링크는 걸렸는데 아직 쓰이지 않은 문서"
      result={await fetchNeededPages()}
      empty="필요한 문서가 없습니다."
    />
  );
}
