import Link from "next/link";

type Action = {
  href: string;
  label: string;
};

// 문서 동작 탭은 곧 URL이다 — 탭 하나가 라우트 하나에 대응한다 (docs/design/07 URL 설계).
export function DocumentActions({
  title,
  current,
}: {
  title: string;
  current: "read" | "edit" | "history" | "discuss";
}) {
  const actions: (Action & { key: typeof current })[] = [
    { key: "read", href: `/w/${title}`, label: "읽기" },
    { key: "edit", href: `/edit/${title}`, label: "편집" },
    { key: "history", href: `/history/${title}`, label: "역사" },
    { key: "discuss", href: `/discuss/${title}`, label: "토론" },
  ];

  return (
    <nav className="flex gap-0.5">
      {actions.map((action) =>
        action.key === current ? (
          <span
            key={action.key}
            aria-current="page"
            className="rounded-t border border-b-0 border-line bg-ground px-2.5 py-1 text-[13px] font-semibold text-ink"
          >
            {action.label}
          </span>
        ) : (
          <Link
            key={action.key}
            href={action.href}
            className="rounded-t border border-b-0 border-transparent px-2.5 py-1 text-[13px] text-muted hover:text-ink"
          >
            {action.label}
          </Link>
        ),
      )}
    </nav>
  );
}
