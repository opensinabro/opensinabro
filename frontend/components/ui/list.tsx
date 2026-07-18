import { cva } from "class-variance-authority";

// 목록은 화면 대부분에서 같은 리듬을 쓴다. preflight가 지운 불릿·여백을 라우트마다
// 다시 적으면 같은 성격의 목록이 화면별로 반 픽셀씩 갈라진다 — 실제로 갈렸다.

export function Rows({
  as = "ul",
  children,
}: {
  as?: "ul" | "ol";
  children: React.ReactNode;
}) {
  const Tag = as;
  return <Tag className="m-0 list-none p-0">{children}</Tag>;
}

export const rowStyle = cva("border-b border-line-soft", {
  variants: {
    shape: {
      /** 제목 한 줄에 짧은 곁들임 — 목록이 길어도 한 줄을 넘기지 않는다. */
      compact: "text-list flex items-baseline gap-2.5 py-2",
      /** 작성자·시각·요약처럼 곁들임이 여럿이라 좁은 화면에서 접혀야 하는 줄. */
      meta: "text-list flex flex-wrap items-baseline gap-x-2.5 gap-y-1 py-2.5",
      /** 본문이 통째로 들어가는 줄 — 안쪽 배치는 내용이 정한다. */
      block: "py-2.5",
    },
  },
  defaultVariants: { shape: "meta" },
});

export function Row({
  shape,
  id,
  children,
}: {
  shape?: "compact" | "meta" | "block";
  /** 목록 안의 한 줄을 링크로 가리켜야 할 때(발언 번호 등). */
  id?: string;
  children: React.ReactNode;
}) {
  return (
    <li id={id} className={rowStyle({ shape })}>
      {children}
    </li>
  );
}
