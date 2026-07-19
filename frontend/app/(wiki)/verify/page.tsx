import { Link } from "@/components/layout/link";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { fetchVerification } from "@/lib/api/account";
import { pageTitle } from "@/lib/site";
import { linkStyle } from "@/components/ui/link";

export const metadata = { title: pageTitle("이메일 확인") };

type PageProps = {
  searchParams: Promise<{ token?: string }>;
};

export default async function VerifyPage({ searchParams }: PageProps) {
  const token = (await searchParams).token?.trim() ?? "";

  if (!token) {
    return (
      <WikiPage header={<PageHeader title="이메일 확인" />}>
        <Notice>메일로 받은 확인 링크를 눌러 주세요.</Notice>
      </WikiPage>
    );
  }

  const result = await fetchVerification(token);
  const verified = result.kind === "found" && result.data.verified;

  return (
    <WikiPage header={<PageHeader title="이메일 확인" />}>
      {verified ? (
        <>
          <Notice>이메일을 확인했습니다. 이제 로그인할 수 있습니다.</Notice>
          <Link href="/login" className={linkStyle({ size: "ui" })}>
            로그인하기
          </Link>
        </>
      ) : (
        <Notice>
          확인 링크가 이미 쓰였거나 유효 기간이 지났습니다. 가입을 다시 시도해
          주세요.
        </Notice>
      )}
    </WikiPage>
  );
}
