import { SignupForm } from "@/components/account/signup-form";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { pageTitle } from "@/lib/site";

export const metadata = { title: pageTitle("계정 만들기") };

export default function SignupPage() {
  return (
    <WikiPage
      header={
        <PageHeader title="계정 만들기" note="이름과 이메일로 가입합니다" />
      }
    >
      <SignupForm />
    </WikiPage>
  );
}
