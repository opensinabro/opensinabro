import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { ConfigForm } from "@/components/operate/config-form";
import { fetchConfiguration } from "@/lib/api/operate";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("위키 설정") };

export default async function ConfigPage() {
  const result = await fetchConfiguration();

  const header = <PageHeader title="위키 설정" note="위키 전역 설정" />;

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 화면을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <Notice>바뀐 설정은 다시 시작한 뒤 화면에 반영됩니다.</Notice>
      <ConfigForm configuration={result.data} />
    </WikiPage>
  );
}
