import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { UploadForm } from "@/components/operate/upload-form";
import { fetchUploadOptions } from "@/lib/api/operate";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("파일 올리기") };

export default async function UploadPage() {
  const result = await fetchUploadOptions();

  const header = (
    <PageHeader title="파일 올리기" note="파일 이름공간에 새 문서를 만듭니다" />
  );

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>파일을 올릴 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <UploadForm options={result.data} />
    </WikiPage>
  );
}
