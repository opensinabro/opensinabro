// 주소 체계는 한곳에서만 만든다. 제목이 경로가 되는 규칙(인코딩·동사 접두사)이
// 라우트마다 흩어지면 문서 안 링크와 주소창 표기가 어긋난다 (docs/architecture.md의 URL 설계).

const preserved = new Set([":", "/", "(", ")"]);

// 렌더러(namumark-backend-namuwiki의 percent_encode)와 같은 규칙 — 대문자 hex를 쓰고
// 이름공간 `:`·하위 문서 `/`·동음이의 `()`는 인코딩하지 않는다. 본문이 방출한 `<a href>`와
// 셸이 만드는 링크가 같은 문자열이어야 같은 문서로 읽힌다.
export function encodeTitle(title: string) {
  return [...title]
    .map((character) =>
      preserved.has(character)
        ? character
        : encodeURIComponent(character).replace(
            /%[0-9a-f]{2}/g,
            (escape) => escape.toUpperCase(),
          ),
    )
    .join("");
}

// 제목은 하위 문서 관습 때문에 `/`를 품는다. Next는 catch-all 조각을 인코딩된 채로
// 넘기므로, 조각마다 풀어 다시 이어 붙여야 `상위/하위`가 한 제목으로 복원된다.
export function joinTitle(segments: string[]) {
  return segments.map(decodeURIComponent).join("/");
}

export const wikiPath = {
  read: (title: string) => `/w/${encodeTitle(title)}`,
  edit: (title: string) => `/edit/${encodeTitle(title)}`,
  history: (title: string) => `/history/${encodeTitle(title)}`,
  discuss: (title: string) => `/discuss/${encodeTitle(title)}`,
  backlink: (title: string) => `/backlink/${encodeTitle(title)}`,
  raw: (title: string) => `/raw/${encodeTitle(title)}`,
  move: (title: string) => `/move/${encodeTitle(title)}`,
  delete: (title: string) => `/delete/${encodeTitle(title)}`,
  blame: (title: string) => `/blame/${encodeTitle(title)}`,
  discussThread: (id: string) => `/thread/${id}`,
  editRequest: (id: string) => `/edit-request/${id}`,
  contributions: (name: string) =>
    `/users/${encodeURIComponent(name)}/contributions`,
  diff: (title: string, revisionId: string) =>
    `/diff/${encodeTitle(title)}?uuid=${revisionId}`,
  rawAt: (title: string, revisionId: string) =>
    `/raw/${encodeTitle(title)}?uuid=${revisionId}`,
  revert: (title: string, revisionId: string) =>
    `/revert/${encodeTitle(title)}?uuid=${revisionId}`,
};
