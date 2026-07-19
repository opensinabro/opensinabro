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

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function daysApart(past: Date, now: Date) {
  const from = new Date(past.getFullYear(), past.getMonth(), past.getDate());
  const to = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  return Math.round((to.getTime() - from.getTime()) / DAY);
}

// 활발한 위키에서는 최근 변경이 대개 같은 날에 몰린다 — 절대 날짜로 적으면 목록의 모든
// 줄이 같은 글자가 되어 폭만 먹고 아무것도 구분하지 못한다. 일주일이 지나면 다시 절대
// 날짜로 돌린다. "37일 전"은 날짜보다 읽기 어렵다.
export function formatRelative(value: string) {
  const past = new Date(value);
  const now = new Date();
  const elapsed = now.getTime() - past.getTime();

  if (elapsed < MINUTE) return "방금";
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)}분 전`;

  // 시간 단위는 경과 시간으로, 일 단위는 달력 날짜로 센다. 30시간 전을 "어제"라 부르면
  // 자정을 두 번 넘긴 그저께가 어제가 된다.
  const days = daysApart(past, now);
  if (days === 0) return `${Math.floor(elapsed / HOUR)}시간 전`;
  if (days === 1) return "어제";
  if (days < 7) return `${days}일 전`;

  return formatDay(value);
}

export function formatBytes(value: number) {
  return `${value.toLocaleString()} B`;
}

export function formatCount(value: number) {
  return value.toLocaleString();
}
