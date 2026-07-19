import type { RenderTree } from "@/lib/namumark/RenderTree";

// 서버 컴포넌트와 클라이언트 컴포넌트가 함께 읽는 응답 타입. 부르는 쪽(server.ts는
// next/headers에 기대고 client.ts는 브라우저에 기댄다)과 갈라 두어야 편집기 같은
// 클라이언트 컴포넌트가 서버 전용 모듈을 타고 들어오지 않는다.

// 셸이 화면마다 필요로 하는 것. 한 번에 받아 두지 않으면 일부 화면만 로그인 상태를
// 모르는 채로 그려진다 (askama 셸에서 겪은 것과 같은 결함 — docs/architecture.md).
export type SessionView = {
  wikiName: string;
  mainDocument: string;
  contentLicense: string;
  userName: string | null;
  unread: number;
};

// 특수 페이지 목록이 공유하는 한 줄. `note`는 화면마다 다른 곁들임(링크된 횟수·
// 바이트 수·마지막 편집일)이고, 곁들일 것이 없는 목록은 빈 문자열을 받는다.
export type TitleEntry = {
  title: string;
  note: string;
};

export type RevisionSummary = {
  id: string;
  sequence: number;
  kind: string;
  author: string;
  comment: string;
  contentBytes: number;
  createdAt: string;
  // 내용이 가려진 리비전인가. 목록에는 남고 원문만 가린다.
  hidden: boolean;
};

export type DocumentView = {
  title: string;
  namespace: string;
  source: string;
  // 본문은 이 렌더 트리로 그린다. 화면 표현은 프론트엔드가 정한다.
  tree: RenderTree;
  revision: RevisionSummary | null;
  backlinkCount: number;
  threadCount: number;
  starred: boolean;
};

export type HistoryView = {
  title: string;
  revisions: RevisionSummary[];
  mayHideRevision: boolean;
};

// 코드값(link·include·redirect…)과 사람이 읽는 이름을 함께 받는다. 라벨만 오면
// 화면이 링크 종류에 따라 갈라 다루지 못한다.
export type BacklinkEntry = {
  title: string;
  kind: string;
  kindLabel: string;
};

export type BacklinkView = {
  title: string;
  entries: BacklinkEntry[];
};

export type RecentChange = {
  title: string;
  revision: RevisionSummary;
};

export type EditView = {
  title: string;
  content: string;
  baseRevision: string;
  editRequestOnly: boolean;
};
