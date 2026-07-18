export const siteName = "오픈시나브로";

// 탭 제목 형식이 라우트마다 갈리지 않도록 한곳에서 만든다.
export function pageTitle(subject: string, section?: string) {
  return section
    ? `${subject} (${section}) - ${siteName}`
    : `${subject} - ${siteName}`;
}
