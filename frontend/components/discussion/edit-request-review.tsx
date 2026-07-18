"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import { FormActions } from "@/components/ui/field";
import {
  acceptEditRequest,
  closeEditRequest,
} from "@/lib/api/discussion-client";
import { wikiPath } from "@/lib/wiki-path";

export function EditRequestReview({ id }: { id: string }) {
  const router = useRouter();
  const [working, setWorking] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function run(action: () => Promise<void>) {
    setWorking(true);
    setProblem(null);

    try {
      await action();
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "처리하지 못했습니다.",
      );
      setWorking(false);
    }
  }

  return (
    <div className="mt-6 border-t border-line pt-5">
      <FormActions>
        <button
          type="button"
          disabled={working}
          onClick={() =>
            run(async () => {
              const title = await acceptEditRequest(id);
              router.push(wikiPath.read(title));
            })
          }
          className={buttonStyle({ tone: "primary" })}
        >
          반영
        </button>
        <button
          type="button"
          disabled={working}
          onClick={() =>
            run(async () => {
              await closeEditRequest(id);
              router.refresh();
              setWorking(false);
            })
          }
          className={buttonStyle()}
        >
          닫기
        </button>
      </FormActions>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </div>
  );
}
