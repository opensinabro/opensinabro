// 검색은 좁은 화면에서도 반드시 닿아야 한다 — 내비 열과 상단 바가 같은 폼을 쓴다.
// 한쪽에만 두면 그 화면 폭에서는 검색으로 가는 길이 사라진다.
export function SearchForm({ id = "search" }: { id?: string }) {
  return (
    <form action="/search" className="relative">
      <input
        id={id}
        name="q"
        placeholder="문서 검색"
        aria-label="문서 검색"
        className="text-fine w-full rounded border border-line bg-ground py-1 pr-7 pl-2.5 text-body placeholder:text-placeholder focus-visible:border-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
      />
      <svg
        aria-hidden="true"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        className="pointer-events-none absolute top-1/2 right-2 size-4 -translate-y-1/2 text-faint"
      >
        <circle cx="7" cy="7" r="4.5" />
        <path d="M10.4 10.4 14 14" />
      </svg>
    </form>
  );
}
