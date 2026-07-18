import { redirect } from "next/navigation";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchRandom } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";
import { wikiPath } from "@/lib/wiki-path";

export const metadata = { title: pageTitle("임의 문서") };

export default async function RandomPage() {
  const result = await fetchRandom();

  if (result.kind === "found" && result.data.title) {
    redirect(wikiPath.read(result.data.title));
  }

  return (
    <WikiPage header={<PageHeader title="임의 문서" />}>
      <Notice>아직 문서가 하나도 없습니다.</Notice>
    </WikiPage>
  );
}
