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
      <header className="flex items-end justify-between gap-4 px-6 pt-4">
        <div>
          <h1 className="m-0 text-[27px] font-extrabold tracking-tight text-ink">
            {title}
          </h1>
          {note && <p className="mt-1 mb-0 text-[12.5px] text-faint">{note}</p>}
        </div>
        {actions}
      </header>
      <div className="border-b border-line" />
    </>
  );
}
