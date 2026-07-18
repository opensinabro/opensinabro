import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { GrantForm } from "@/components/operate/grant-form";
import { fetchGrantOptions } from "@/lib/api/operate";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("권한 부여") };

export default async function GrantPage() {
  const result = await fetchGrantOptions();

  const header = (
    <PageHeader title="권한 부여" note="사용자에게 권한을 주거나 거둡니다" />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 화면을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  if (result.data.permissions.length === 0) {
    return (
      <WikiPage header={header}>
        <Notice>줄 수 있는 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <GrantForm permissions={result.data.permissions} />
    </WikiPage>
  );
}
