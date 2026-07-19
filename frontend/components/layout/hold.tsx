// 가운데 정주형의 유일한 자. 헤더 띠·본문·푸터 띠가 모두 이걸 통과해야 왼쪽 끝이
// 한 줄로 선다. 배경은 이 바깥의 부모가 화면 끝까지 칠하고, 내용만 여기 맞춘다.
export function Hold({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`mx-auto w-full max-w-hold px-4 sm:px-6 ${className ?? ""}`}
    >
      {children}
    </div>
  );
}
