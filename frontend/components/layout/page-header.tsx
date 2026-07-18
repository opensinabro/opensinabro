// 모든 화면의 머리는 이 한 벌이다 — 제목·보조 설명·동작 자리. 라우트가 직접 마크업을
// 짜면 화면마다 제목 크기와 여백이 어긋난다.
export function PageHeader({
  title,
  note,
  actions,
}: {
  title: string;
  note?: string;
  actions?: React.ReactNode;
}) {
  return (
    <>
      <header className="flex flex-wrap items-end justify-between gap-x-4 gap-y-2 px-4 pt-4 sm:px-6">
        <div className="min-w-0">
          <h1 className="text-title m-0 font-extrabold tracking-tight text-ink">
            {title}
          </h1>
          {note && <p className="text-note mt-1 mb-0 text-faint">{note}</p>}
        </div>
        {actions}
      </header>
      <div className="border-b border-line" />
    </>
  );
}
