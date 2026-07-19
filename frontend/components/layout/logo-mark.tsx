// 위키 이름은 관리자가 바꿀 수 있는 서버 설정이지만 이 마크는 엔진의 표식이라 함께
// 바뀌지 않는다. 그래서 이름 옆에 서되 이름을 대신하지는 않는다.
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 64 64"
      aria-hidden="true"
      fill="none"
      stroke="currentColor"
      strokeWidth={3.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <path d="M22 11h-8v42h8M42 11h8v42h-8" />
      <path
        d="M32 47c-7-6-7-18 0-27 7 9 7 21 0 27z"
        fill="currentColor"
        stroke="none"
      />
    </svg>
  );
}
