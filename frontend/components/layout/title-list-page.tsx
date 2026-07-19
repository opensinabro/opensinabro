import { Link } from "@/components/layout/link";
import { TitleList } from "@/components/document/title-list";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { linkStyle } from "@/components/ui/link";
import type { Fetched } from "@/lib/api/fetch";
import type { TitleEntry } from "@/lib/api/types";

// 특수 페이지 여섯 개가 "제목 목록 한 벌"이라는 같은 화면이다. 라우트마다 같은 분기를
// 되풀이하지 않도록 제목·안내와 조회 결과만 받아 그린다.
export function TitleListPage({
  title,
  note,
  result,
  empty,
  actions,
}: {
  title: string;
  note?: string;
  result: Fetched<{ entries: TitleEntry[] }>;
  empty: string;
  actions?: React.ReactNode;
}) {
  const header = <PageHeader title={title} note={note} actions={actions} />;

  if (result.kind === "unauthorized") {
    return (
      <WikiPage header={header}>
        <Notice>
          로그인해야 볼 수 있는 화면입니다.{" "}
          <Link href="/login" className={linkStyle()}>
            로그인하기
          </Link>
        </Notice>
      </WikiPage>
    );
  }

  if (result.kind !== "found") {
    return (
      <WikiPage header={header}>
        <Notice>이 목록을 볼 권한이 없습니다.</Notice>
      </WikiPage>
    );
  }

  return (
    <WikiPage header={header}>
      <TitleList entries={result.data.entries} empty={empty} />
    </WikiPage>
  );
}
