import { NavigationColumn } from "@/components/navigation-column";

// 확정안 C1의 3열 정주형. 좌측 내비는 이 레이아웃이 그려 라우트 사이에서 재사용되고,
// 본문과 우측 정보 열은 페이지가 형제 요소 둘로 돌려준다 — 두 열이 같은 그리드의
// 항목이어야 높이가 맞물리기 때문이다 (docs/design/07 M7).
export default function WikiLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <div className="grid min-h-full grid-cols-1 lg:grid-cols-[186px_minmax(0,1fr)]">
      <div className="hidden lg:block">
        <NavigationColumn wikiName="오픈시나브로" />
      </div>
      <div className="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_232px]">
        {children}
      </div>
    </div>
  );
}
