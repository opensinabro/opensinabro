"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { DocumentActions } from "@/components/document-actions";

// 없는 문서는 안내 화면을 보이되 상태 코드는 404여야 한다 (docs/design/07 URL 설계).
// 그러려면 페이지가 notFound()로 넘겨야 하고, 그 시점에는 제목이 경로에만 남는다.
export default function DocumentNotFound() {
  const title = decodeURIComponent(usePathname().replace(/^\/w\//, ""));

  return (
    <article className="min-w-0">
      <header className="flex items-end justify-between gap-4 px-6 pt-4">
        <h1 className="m-0 text-[27px] font-extrabold tracking-tight text-ink">
          {title}
        </h1>
        <DocumentActions title={title} current="read" />
      </header>
      <div className="border-b border-line" />

      <p className="px-6 pt-4 text-muted">
        이 문서는 아직 없습니다.{" "}
        <Link href={`/edit/${title}`} className="text-link hover:underline">
          지금 만들기
        </Link>
      </p>
    </article>
  );
}
