import { joinTitle } from "@/lib/wiki-path";

// 문서 동작 라우트는 전부 catch-all 제목 하나를 받는다. 그 서명과 제목 복원을
// 한곳에 두어 라우트가 늘어도 같은 방식으로 제목을 얻게 한다.
export type DocumentRouteProps = {
  params: Promise<{ title: string[] }>;
};

export async function routeTitle(params: DocumentRouteProps["params"]) {
  return joinTitle((await params).title);
}
