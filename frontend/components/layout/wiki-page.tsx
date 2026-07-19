import { Suspense } from "react";
import { Hold } from "@/components/layout/hold";
import {
  TableOfContentsRail,
  type TableOfContentsRailEntry,
} from "@/components/layout/toc-rail";
import { WikiColumn } from "@/components/layout/wiki-column";

// 가운데 정주형에서 한 화면이 차지하는 자리를 이 컴포넌트가 통째로 소유한다 —
// 본문과 우측 위키 열, 그리고 화면 오른쪽 끝 스크롤바 옆에 서는 목차 축까지.
//
// 우측 열은 페이지가 넘기는 것이 아니라 여기가 직접 그린다. 화면마다 넘기게 하면
// 넘기는 것을 잊은 화면에서 열이 사라져, 라우트를 옮길 때마다 본문 폭이 흔들린다.
//
// 목차는 문서 화면만 넘긴다. 목록·양식 화면은 문단이 없어 축이 설 이유가 없다.
export function WikiPage({
  header,
  toc,
  variant = "prose",
  children,
}: {
  header: React.ReactNode;
  /** 문서의 문단 목록. 없거나 비면 우측 축을 그리지 않는다. */
  toc?: TableOfContentsRailEntry[];
  /** `full`은 작성자 보기처럼 폭을 다 쓰는 화면 — 우측 열도 축도 걷는다. */
  variant?: "prose" | "full";
  children: React.ReactNode;
}) {
  if (variant === "full") {
    return (
      <Hold className="flex min-h-0 flex-1 flex-col pt-5 pb-7">
        <main id="content" className="flex min-w-0 flex-1 flex-col">
          {header}
          <div className="flex min-h-0 flex-1 flex-col pt-4">{children}</div>
        </main>
      </Hold>
    );
  }

  return (
    <div className="relative flex-1">
      {toc && toc.length > 0 && <TableOfContentsRail entries={toc} />}

      <Hold className="grid grid-cols-1 gap-x-7 pt-5 pb-7 column:grid-cols-[minmax(0,1fr)_260px]">
        <main id="content" className="flex min-w-0 flex-col">
          {header}
          <div className="pt-5">{children}</div>
        </main>

        {/* 위키 열은 문서 조회와 별개로 받는다. 기다리게 두면 문서가 늦어지므로
            본문보다 늦게 도착하도록 떼어 둔다. */}
        <Suspense fallback={null}>
          <WikiColumn />
        </Suspense>
      </Hold>
    </div>
  );
}
