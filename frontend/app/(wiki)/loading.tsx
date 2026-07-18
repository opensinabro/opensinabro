// 모든 조회가 no-store라 라우트를 옮길 때마다 axum 왕복을 기다린다. 이 자리가
// 없으면 그동안 이전 화면이 그대로 남아 클릭이 먹지 않은 것처럼 보인다.
export default function WikiLoading() {
  return (
    <div className="px-4 pt-4 sm:px-6">
      <p role="status" className="text-note m-0 py-2 text-faint">
        불러오는 중…
      </p>
    </div>
  );
}
