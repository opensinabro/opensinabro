import { cva } from "class-variance-authority";

// 좌측 내비의 묶음과 우측 정보 열의 묶음은 같은 형식이다 — 제목 + 내용. 같은 컴포넌트를
// 쓰게 해 두 열의 리듬이 갈라지지 않게 한다.
//
// 다만 제목의 크기는 갈린다. 내비의 묶음 이름은 이름표에 가까워 작게 두지만, 우측 열의
// 묶음은 그 자체가 읽을거리를 이끄는 제목이라 본문 크기에 가깝게 선다.
const headingStyle = cva("font-bold", {
  variants: {
    size: {
      label: "text-label tracking-[0.12em] text-faint uppercase",
      title: "text-ui text-body",
    },
  },
  defaultVariants: { size: "label" },
});

export function Section({
  label,
  size,
  action,
  children,
}: {
  label: string;
  size?: "label" | "title";
  /** 묶음 전체로 가는 링크처럼, 제목 줄 오른쪽 끝에 서는 것. */
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section>
      <div className="mb-2 flex items-baseline justify-between gap-2">
        <h2 className={headingStyle({ size })}>{label}</h2>
        {action}
      </div>
      {children}
    </section>
  );
}

export function LinkList({ children }: { children: React.ReactNode }) {
  return (
    <ul className="m-0 flex list-none flex-col gap-1.5 p-0">{children}</ul>
  );
}
