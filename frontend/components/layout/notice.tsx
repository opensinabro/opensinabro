// 권한 거부·빈 목록·저장 실패는 화면마다 문구만 다르고 자리와 톤은 같다.
// 한곳에 모아 두지 않으면 라우트가 늘 때마다 안내 마크업이 한 벌씩 늘어난다.

export function Notice({ children }: { children: React.ReactNode }) {
  return <p className="text-note m-0 py-2 text-muted">{children}</p>;
}

const tones = {
  neutral: "border-line bg-ground-sub text-muted",
  danger: "border-danger-line bg-danger-wash text-danger-ink",
} as const;

export function Alert({
  tone = "neutral",
  children,
}: {
  tone?: keyof typeof tones;
  children: React.ReactNode;
}) {
  return (
    <p
      className={`text-note m-0 mt-3 rounded border px-3 py-2 ${tones[tone]}`}
      role={tone === "danger" ? "alert" : undefined}
    >
      {children}
    </p>
  );
}
