"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import { FormActions } from "@/components/ui/field";
import { batchRevert } from "@/lib/api/operate-client";

export function BatchRevertButton({ author }: { author: string }) {
  const router = useRouter();
  const [reverting, setReverting] = useState(false);
  const [reverted, setReverted] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  async function revert() {
    setReverting(true);
    setProblem(null);

    try {
      const count = await batchRevert(author);
      setReverted(count);
      // 되돌린 뒤에는 대상 목록이 달라진다 — 서버가 다시 세도록 한다.
      router.refresh();
    } catch (error) {
      setProblem(
        error instanceof Error
          ? error.message
          : "일괄 되돌리기를 하지 못했습니다.",
      );
    } finally {
      setReverting(false);
    }
  }

  return (
    <div className="mt-5">
      <FormActions>
        <button
          type="button"
          onClick={revert}
          disabled={reverting}
          className={buttonStyle({ tone: "primary" })}
        >
          {reverting ? "되돌리는 중" : "모두 되돌리기"}
        </button>
      </FormActions>
      {reverted !== null && <Alert>{reverted}개 문서를 되돌렸습니다.</Alert>}
      {problem && <Alert tone="danger">{problem}</Alert>}
    </div>
  );
}
