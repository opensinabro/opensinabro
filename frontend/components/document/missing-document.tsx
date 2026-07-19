"use client";

import { Link } from "@/components/layout/link";
import { usePathname } from "next/navigation";
import { DocumentActions } from "@/components/document/document-actions";
import { Notice } from "@/components/layout/notice";
import { PageHeader } from "@/components/layout/page-header";
import { linkStyle } from "@/components/ui/link";
import { wikiPath } from "@/lib/wiki-path";

// 없는 문서 화면은 제목을 경로에서만 얻을 수 있어 클라이언트에서 읽어야 한다.
// 그렇다고 화면 전체를 클라이언트로 만들면 WikiPage가 그리는 위키 열(서버에서
// 받아 오는 것)이 함께 끌려 들어가 빌드가 깨진다 — 제목이 필요한 두 조각만 뗀다.
function useMissingTitle() {
  // 훅을 부르므로 이름도 훅 규칙을 따른다 — 두 조각이 같은 경로 해석을 쓰게 하려고 뗐다.
  return decodeURIComponent(usePathname().replace(/^\/w\//, ""));
}

export function MissingDocumentHeader() {
  const title = useMissingTitle();

  return (
    <PageHeader
      title={title}
      actions={<DocumentActions title={title} current="read" />}
    />
  );
}

export function MissingDocumentNotice() {
  const title = useMissingTitle();

  // 만들기만 주면 글을 쓸 사람에게만 길이 있다. 대부분은 오타이거나 제목을 조금
  // 다르게 아는 경우라, 같은 제목으로 본문을 뒤지는 길도 함께 준다.
  return (
    <Notice>
      이 문서는 아직 없습니다.{" "}
      <Link href={wikiPath.edit(title)} className={linkStyle()}>
        지금 만들기
      </Link>{" "}
      ·{" "}
      <Link href={wikiPath.search(title)} className={linkStyle()}>
        이 제목으로 검색
      </Link>
    </Notice>
  );
}
