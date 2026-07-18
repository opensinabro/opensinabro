export type DiffLine = {
  kind: "inserted" | "deleted" | "context";
  text: string;
};

const marks = {
  inserted: { sign: "+", className: "bg-accent-wash text-accent-deep" },
  deleted: { sign: "-", className: "bg-danger-wash text-danger-ink" },
  context: { sign: " ", className: "text-muted" },
} as const;

// 서버가 주는 diff는 hunk 경계를 버린 평탄한 줄 배열이라(diffy 패치를 flat_map으로
// 편다), 화면도 줄 번호 없는 통합 한 칸으로 그린다.
export function DiffView({ lines }: { lines: DiffLine[] }) {
  if (lines.length === 0) {
    return <p className="text-note m-0 py-2 text-muted">달라진 곳이 없습니다.</p>;
  }

  return (
    <pre className="m-0 overflow-x-auto rounded border border-line font-mono text-note leading-[1.7]">
      {lines.map((line, index) => {
        const mark = marks[line.kind];
        return (
          <span key={index} className={`block px-3 ${mark.className}`}>
            {mark.sign}
            {line.text}
          </span>
        );
      })}
    </pre>
  );
}
