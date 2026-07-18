import { LoginForm } from "@/components/account/login-form";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("로그인") };

export default function LoginPage() {
  return (
    <WikiPage
      header={<PageHeader title="로그인" note="계정으로 들어가기" />}
    >
      <LoginForm />
    </WikiPage>
  );
}
