// 폼은 화면마다 필드 몇 개와 단추 하나라는 같은 모양이다. 라벨 위치·간격·오류 표시를
// 여기 모아 두어야 로그인·이동·삭제·올리기가 서로 다른 폼처럼 보이지 않는다.

export function Field({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={htmlFor} className="text-note font-semibold text-ink">
        {label}
      </label>
      {children}
      {hint && <p className="text-fine m-0 text-faint">{hint}</p>}
    </div>
  );
}

export const inputStyle =
  "text-note w-full rounded border border-line bg-ground px-2.5 py-1.5 text-body focus:border-accent focus:outline-none";

export function FormLayout({ children }: { children: React.ReactNode }) {
  return <div className="flex max-w-[420px] flex-col gap-3.5">{children}</div>;
}

export function FormActions({ children }: { children: React.ReactNode }) {
  return <div className="mt-1 flex items-center gap-2">{children}</div>;
}
