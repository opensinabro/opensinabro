"use client";

import { usePathname } from "next/navigation";
import { ErrorScreen } from "@/components/layout/error-screen";
import { Hold } from "@/components/layout/hold";
import { readableAddress } from "@/lib/wiki-path";

// 위키 안의 화면이 notFound()로 넘겼을 때. 이것이 없으면 문서 화면이 아닌 라우트
// (역사·역링크·토론 …)의 404가 셸 바깥의 root not-found로 떨어져, 헤더도 푸터도 없는
// 화면이 된다 — 문서 없음만 셸을 지키고 나머지는 못 지키는 셈이 된다.
//
// 없는 *문서*는 w/[...title]/not-found.tsx가 먼저 받는다. 그쪽이 제목을 알아
// "지금 만들기"를 줄 수 있으므로, 여기는 그 밖의 대상만 맡는다.
export default function WikiNotFound() {
  const address = readableAddress(usePathname());

  return (
    <Hold className="flex flex-1 flex-col pt-5 pb-7">
      <main id="content" className="flex min-w-0 flex-col">
        <ErrorScreen
          title="찾을 수 없습니다"
          subject={address}
          description="가리키는 대상이 없습니다. 지워졌거나 옮겨졌을 수 있습니다."
          actions={[
            { label: "대문으로", href: "/" },
            { label: "최근 변경", href: "/recent-changes" },
          ]}
        />
      </main>
    </Hold>
  );
}
