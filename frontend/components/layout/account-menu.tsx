"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { logOut } from "@/lib/api/client";
import type { SessionView } from "@/lib/api/types";

export function AccountMenu({ session }: { session: SessionView }) {
  const router = useRouter();

  if (!session.userName) {
    return (
      <div className="text-ui flex flex-col gap-px">
        <Link
          href="/login"
          className="rounded px-1.5 py-1 text-body hover:bg-ground-deep"
        >
          로그인
        </Link>
        <Link
          href="/signup"
          className="rounded px-1.5 py-1 text-body hover:bg-ground-deep"
        >
          계정 만들기
        </Link>
      </div>
    );
  }

  return (
    <div className="text-ui flex flex-col gap-px">
      <Link
        href={`/users/${encodeURIComponent(session.userName)}`}
        className="rounded px-1.5 py-1 font-semibold text-ink hover:bg-ground-deep"
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
        className="rounded px-1.5 py-1 text-left text-muted hover:bg-ground-deep"
      >
        로그아웃
      </button>
    </div>
  );
}
