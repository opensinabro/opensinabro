import { redirect } from "next/navigation";
import { wikiPath } from "@/lib/wiki-path";

type PageProps = {
  params: Promise<{ name: string }>;
};

// 사용자의 홈은 사용자 이름공간의 문서다 — 별도 화면을 두지 않고 그리로 넘긴다.
export default async function UserPage({ params }: PageProps) {
  const { name } = await params;
  redirect(wikiPath.read(`사용자:${decodeURIComponent(name)}`));
}
