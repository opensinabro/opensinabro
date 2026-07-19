// 모든 화면의 머리는 이 한 벌이다 — 제목·보조 설명·탭·도구 줄. 라우트가 직접 마크업을
// 짜면 화면마다 제목 크기와 여백이 어긋난다. 좌우 여백은 Hold가, 셸 아래로 띄우는
// 윗여백은 그 Hold를 세우는 쪽(WikiPage)이 이미 주므로 여기서 다시 주지 않는다 —
// 여기서 주면 본문과 우측 위키 열이 서로 다른 높이에서 시작한다.
export function PageHeader({
  title,
  note,
  actions,
  toolbar,
}: {
  title: string;
  note?: string;
  /** 문서 탭처럼 제목과 같은 줄에 서는 것. */
  actions?: React.ReactNode;
  /** 탭 오른쪽에 붙는 도구. 탭과 한 덩어리로 읽히도록 같은 줄에 세운다. */
  toolbar?: React.ReactNode;
}) {
  return (
    <header>
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2 pb-1.5">
        <div className="min-w-0">
          <h1 className="text-title m-0 font-extrabold tracking-tight text-ink">
            {title}
          </h1>
          {note && <p className="text-note mt-1 mb-0 text-faint">{note}</p>}
        </div>
        {(actions || toolbar) && (
          <div className="flex flex-wrap items-center gap-1.5">
            {actions}
            {toolbar}
          </div>
        )}
      </div>
    </header>
  );
}
