import { NavigationColumn } from "@/components/layout/navigation-column";
import { SiteFooter } from "@/components/layout/site-footer";
import { fetchSession } from "@/lib/api/server";

// 확정안 C1의 3열 정주형. 좌측 내비는 이 레이아웃이 그려 라우트 사이에서 재사용되고,
// 본문과 우측 정보 열은 페이지가 <WikiPage>로 채운다 — 두 열이 같은 그리드의 항목이어야
// 높이가 맞물리기 때문이다 (docs/architecture.md).
//
// 세션은 여기서 한 번만 읽어 내려보낸다. 화면마다 따로 물으면 일부 화면만 로그인
// 상태를 모르는 채로 그려진다.
export default async function WikiLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  const session = await fetchSession();

  // 내비와 푸터가 화면 아래까지 닿아야 짧은 문서에서 셸이 허공에 뜬 것처럼 보이지
  // 않는다. 부모 높이가 내용에 따라 접히므로 뷰포트 높이를 직접 잡는다.
  return (
    <div className="grid min-h-dvh grid-cols-1 lg:grid-cols-[186px_minmax(0,1fr)]">
      <div className="hidden lg:block">
        <NavigationColumn session={session} />
      </div>
      <div className="flex min-w-0 flex-col">
        <div className="grid flex-1 grid-cols-1 xl:grid-cols-[minmax(0,1fr)_232px]">
          {children}
        </div>
        <SiteFooter contentLicense={session.contentLicense} />
      </div>
    </div>
  );
}
