import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { Section } from "@/components/ui/section";
import { fetchLicense } from "@/lib/api/special";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("라이선스") };

export default async function LicensePage() {
  const result = await fetchLicense();

  if (result.kind !== "found") {
    return (
      <WikiPage header={<PageHeader title="라이선스" />}>
        <Notice>라이선스 안내를 읽지 못했습니다.</Notice>
      </WikiPage>
    );
  }

  const { engineNotice, contentLicense } = result.data;

  return (
    <WikiPage header={<PageHeader title="라이선스" />}>
      <div className="flex flex-col gap-5">
        <Section label="문서 내용">
          <p className="text-list m-0 text-body">
            이 위키의 문서 내용은 {contentLicense}를 따릅니다.
          </p>
        </Section>
        <Section label="엔진">
          <p className="text-list m-0 text-body">{engineNotice}</p>
        </Section>
      </div>
    </WikiPage>
  );
}
