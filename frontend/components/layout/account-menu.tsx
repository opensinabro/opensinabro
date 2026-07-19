"use client";

import { Link } from "@/components/layout/link";
import { useRouter } from "next/navigation";
import { logOut } from "@/lib/api/client";
import type { SessionView } from "@/lib/api/types";

// 헤더 한 줄의 끝자리. 세로 목록이던 것이 가로로 누웠으므로 로그아웃은 이름 옆에
// 나란히 선다 — 여닫이를 하나 더 두면 헤더에 겹이 늘고, 여기 담을 것은 둘뿐이다.
export function AccountMenu({ session }: { session: SessionView }) {
  const router = useRouter();

  if (!session.userName) {
    return (
      <div className="text-ui flex shrink-0 items-center gap-0.5">
        <Link
          href="/login"
          className="rounded px-1.5 py-1 text-body hover:bg-ground-deep"
        >
          로그인
        </Link>
        <Link
          href="/signup"
          className="hidden rounded px-1.5 py-1 text-muted hover:bg-ground-deep sm:block"
        >
          계정 만들기
        </Link>
      </div>
    );
  }

  return (
    <div className="text-ui flex shrink-0 items-center gap-0.5">
      <Link
        href={`/users/${encodeURIComponent(session.userName)}`}
        className="max-w-[120px] truncate rounded px-1.5 py-1 font-semibold text-ink hover:bg-ground-deep"
      >
        {session.userName}
      </Link>
      <button
        type="button"
        onClick={async () => {
          await logOut();
          // 셸이 서버에서 그려지므로 로그아웃 뒤에는 다시 받아야 상태가 바뀐다.
          router.refresh();
        }}
        className="hidden rounded px-1.5 py-1 text-muted hover:bg-ground-deep sm:block"
      >
        로그아웃
      </button>
    </div>
  );
}
