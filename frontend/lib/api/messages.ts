// 서버는 권한·존재 여부·실패한 계층을 기계가 읽을 토큰으로 낸다. 그대로 보이면
// 한국어 화면에 영단어가 튀어나오므로 사람이 읽을 문구로 옮긴다.
//
// 조회(서버 컴포넌트)와 변경(브라우저)이 같은 토큰을 받으므로 사전도 하나여야 한다 —
// 둘로 나뉘면 같은 실패가 화면마다 다른 문장으로 뜬다.
const machineMessages: Record<string, string> = {
  forbidden: "권한이 없습니다.",
  unauthorized: "로그인이 필요합니다.",
  not_found: "대상을 찾지 못했습니다.",
  invalid_credentials: "계정 또는 비밀번호가 올바르지 않습니다.",

  // 서버가 어느 계층에서 실패했는지까지 알려 온다. 전부 한 문장으로 받으면 사용자가
  // 다시 시도할지 신고할지 판단할 근거가 사라진다.
  document_failed: "문서를 읽고 쓰는 중 문제가 생겼습니다.",
  search_failed: "검색하는 중 문제가 생겼습니다.",
  account_failed: "계정을 처리하는 중 문제가 생겼습니다.",
  authorization_failed: "권한을 확인하는 중 문제가 생겼습니다.",
  discussion_failed: "토론을 처리하는 중 문제가 생겼습니다.",
  storage_failed: "저장소에 닿지 못했습니다.",
  upload_failed: "올린 파일을 읽지 못했습니다.",
  session_failed: "로그인 상태를 확인하지 못했습니다.",
};

// 서버가 사전에 없는 문장을 직접 낼 때도 있다(입력 반려). 그때는 그 문장이 이미
// 사람이 읽을 것이므로 그대로 쓴다.
export function humanMessage(token: string | undefined, fallback: string) {
  if (!token) return fallback;
  return machineMessages[token] ?? token;
}
