const revisionKindLabels: Record<string, string> = {
  create: "새 문서",
  edit: "편집",
  move: "이동",
  delete: "삭제",
  restore: "복원",
  revert: "되돌림",
  import: "가져오기",
};

export function revisionKindLabel(kind: string) {
  return revisionKindLabels[kind] ?? kind;
}

export function formatMoment(value: string) {
  return new Date(value).toLocaleString("ko-KR", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatDay(value: string) {
  return new Date(value).toLocaleDateString("ko-KR", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

export function formatBytes(value: number) {
  return `${value.toLocaleString()} B`;
}

export function formatCount(value: number) {
  return value.toLocaleString();
}
