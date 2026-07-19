import { NavigationProgress } from "@/components/layout/navigation-progress";
import { SiteFooter } from "@/components/layout/site-footer";
import { SiteHeader } from "@/components/layout/site-header";
import { FootnotePreviewPosition } from "@/components/namumark/footnote-preview-position";
import { fetchSession } from "@/lib/api/server";

// 가운데 정주형. 셸의 크롬은 위아래 띠 두 개뿐이고, 띠의 배경만 화면 끝까지 가고
// 내용은 컨테이너(Hold) 안에 선다 — 넓은 모니터에서 셸이 한가운데 떠 있는 것처럼
// 보이지 않게 하는 것이 띠의 일이다.
//
// 좌측 내비와 서랍은 없앴다. 갈 곳은 헤더 한 줄이 전부 쥐고 있어, 화면 폭이 달라져도
// 길이 사라지지 않는다 — 좁아지면 링크가 "더보기" 안으로 들어갈 뿐이다.
//
// 세션은 여기서 한 번만 읽어 내려보낸다. 화면마다 따로 물으면 일부 화면만 로그인
// 상태를 모르는 채로 그려진다.
export default async function WikiLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  const session = await fetchSession();

  // 푸터가 화면 아래까지 닿아야 짧은 문서에서 셸이 허공에 뜬 것처럼 보이지 않는다.
  // 부모 높이가 내용에 따라 접히므로 뷰포트 높이를 직접 잡는다.
  return (
    <div className="flex min-h-dvh flex-col">
      {/* 라우트 그룹에 loading.tsx를 두지 않는다. 두면 이동하는 순간 본문 자리가
          비어 로딩 문구가 들어섰다가 다시 본문으로 바뀌어, 40ms짜리 이동에도 화면이
          두 번 갈린다. 대신 응답이 다 온 뒤 한 번에 갈아 끼우고, 그동안 눌린 것을
          알리는 일만 이 띠가 맡는다. */}
      <NavigationProgress />

      {/* 목차 축보다 앞에 온다 — 목차가 본문 진입을 막지 않는다. */}
      <a
        href="#content"
        className="text-ui sr-only rounded border border-line bg-ground px-3 py-1 focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-30"
      >
        본문으로 건너뛰기
      </a>

      <SiteHeader session={session} />

      <div className="flex flex-1 flex-col">{children}</div>

      <SiteFooter contentLicense={session.contentLicense} />

      <FootnotePreviewPosition />
    </div>
  );
}
