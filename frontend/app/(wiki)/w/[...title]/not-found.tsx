"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { WikiPage } from "@/components/layout/wiki-page";
import { wikiPath } from "@/lib/wiki-path";
import { linkStyle } from "@/components/ui/link";

// 없는 문서는 안내 화면을 보이되 상태 코드는 404여야 한다 (docs/architecture.md의 URL 설계).
// 그러려면 페이지가 notFound()로 넘겨야 하고, 그 시점에는 제목이 경로에만 남는다.
export default function DocumentNotFound() {
  const title = decodeURIComponent(usePathname().replace(/^\/w\//, ""));

  return (
    <WikiPage
      header={
        <PageHeader
          title={title}
          actions={<DocumentActions title={title} current="read" />}
        />
      }
    >
      <Notice>
        이 문서는 아직 없습니다.{" "}
        <Link href={wikiPath.edit(title)} className={linkStyle()}>
          지금 만들기
        </Link>
      </Notice>
    </WikiPage>
  );
}
