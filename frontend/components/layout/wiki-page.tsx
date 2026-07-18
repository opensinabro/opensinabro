// 3열 정주형에서 페이지가 채우는 두 칸(본문·정보)을 이 컴포넌트가 소유한다.
// 라우트가 그리드 항목을 직접 내놓으면 "형제 둘을 돌려준다"는 규칙이 암묵적이 되어,
// 정보 열이 없는 화면마다 col-span을 손으로 붙이거나 빠뜨리게 된다 (docs/architecture.md).
//
// 정보 열은 좁은 화면에서 걷히고 본문 하단으로 내려간다 — 같은 노드를 두 자리에
// 그리므로 두 배치가 어긋날 수 없다.
export function WikiPage({
  header,
  aside,
  variant = "prose",
  children,
}: {
  header: React.ReactNode;
  aside?: React.ReactNode;
  /** `full`은 편집기처럼 화면 폭을 다 쓰는 화면 — 본문 열의 읽기 폭 제한을 걷는다. */
  variant?: "prose" | "full";
  children: React.ReactNode;
}) {
  const body =
    variant === "prose"
      ? "max-w-[900px] px-4 pt-4 sm:px-6"
      : "flex min-h-0 flex-1 flex-col";

  return (
    <>
      <main
        id="content"
        className={`flex min-w-0 flex-col pb-7 ${aside ? "" : "xl:col-span-2"}`}
      >
        {header}
        <div className={body}>
          {children}
          {aside && (
            <div className="mt-8 border-t border-line pt-4 xl:hidden">
              {aside}
            </div>
          )}
        </div>
      </main>

      {aside && (
        <aside className="hidden border-l border-line px-4 py-4 xl:block">
          <div className="sticky top-4">{aside}</div>
        </aside>
      )}
    </>
  );
}
