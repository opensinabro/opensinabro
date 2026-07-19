import { NavigationProgress } from "@/components/layout/navigation-progress";
import { FootnotePreviewPosition } from "@/components/namumark/footnote-preview-position";

// 편집기는 셸을 쓰지 않는다. 머리띠도 푸터도 가운데 정주 컨테이너도 없이 뷰포트를
// 통째로 가져간다 — 편집 중에 갈 곳은 "나가기" 하나뿐이므로 내비를 세울 이유가 없고,
// 그 자리를 원문과 미리보기가 가져가는 편이 낫다.
//
// 그래서 (wiki)가 아니라 자기 그룹에 산다. 라우트 그룹은 주소에 나타나지 않으므로
// `/edit/…`은 그대로다.
//
// 높이를 `h-dvh`로 못박고 넘침을 막는다. 안쪽 두 열이 각자 스크롤해야 하는데,
// 바깥이 내용만큼 늘어나면 문서 전체가 한 번 더 스크롤되어 축이 둘이 된다.
export default function EditorLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <div className="flex h-dvh flex-col overflow-hidden">
      <NavigationProgress />
      {children}
      <FootnotePreviewPosition />
    </div>
  );
}
