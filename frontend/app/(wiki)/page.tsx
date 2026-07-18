import { redirect } from "next/navigation";
import { fetchSession } from "@/lib/api/server";
import { wikiPath } from "@/lib/wiki-path";

// 대문이 어느 문서인지는 위키 설정이 정한다 — 제목을 여기 박아 두면 설정을 바꿔도
// 첫 화면만 옛 문서로 간다.
export default async function IndexPage() {
  const session = await fetchSession();
  redirect(wikiPath.read(session.mainDocument));
}
