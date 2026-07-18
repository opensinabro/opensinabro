// 검색은 좁은 화면에서도 반드시 닿아야 한다 — 내비 열과 상단 바가 같은 폼을 쓴다.
// 한쪽에만 두면 그 화면 폭에서는 검색으로 가는 길이 사라진다.
export function SearchForm({ id = "search" }: { id?: string }) {
  return (
    <form action="/search">
      <input
        id={id}
        name="q"
        placeholder="문서 검색"
        aria-label="문서 검색"
        className="text-fine w-full rounded border border-line bg-ground px-2.5 py-1 text-body placeholder:text-faint focus-visible:border-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
      />
    </form>
  );
}
