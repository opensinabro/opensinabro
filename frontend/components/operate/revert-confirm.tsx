"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import { FormActions } from "@/components/ui/field";
import { revertDocument } from "@/lib/api/operate-client";
import { wikiPath } from "@/lib/wiki-path";

export function RevertConfirm({
  title,
  revisionId,
}: {
  title: string;
  revisionId: string;
}) {
  const router = useRouter();
  const [reverting, setReverting] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function revert() {
    setReverting(true);
    setProblem(null);

    try {
      const reverted = await revertDocument(title, revisionId);
      router.push(wikiPath.read(reverted));
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "되돌리지 못했습니다.",
      );
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
          {reverting ? "되돌리는 중" : "되돌리기"}
        </button>
      </FormActions>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </div>
  );
}
