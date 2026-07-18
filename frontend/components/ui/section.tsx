// 좌측 내비의 묶음과 우측 정보 열의 묶음은 같은 형식이다 — 작은 대문자 제목 + 내용.
// 같은 컴포넌트를 쓰게 해 두 열의 리듬이 갈라지지 않게 한다.
export function Section({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <h2 className="text-label mb-2 font-bold tracking-[0.12em] text-faint uppercase">
        {label}
      </h2>
      {children}
    </section>
  );
}

export function LinkList({ children }: { children: React.ReactNode }) {
  return (
    <ul className="m-0 flex list-none flex-col gap-1.5 p-0">{children}</ul>
  );
}
