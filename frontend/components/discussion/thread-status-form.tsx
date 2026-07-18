"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import { FormActions, inputStyle } from "@/components/ui/field";
import { changeThreadStatus } from "@/lib/api/discussion-client";

const choices = [
  { value: "normal", label: "정상" },
  { value: "pause", label: "중단" },
  { value: "close", label: "닫힘" },
];

export function ThreadStatusForm({
  threadId,
  status,
}: {
  threadId: string;
  status: string;
}) {
  const router = useRouter();
  const [next, setNext] = useState(
    choices.some((choice) => choice.value === status) ? status : "normal",
  );
  const [changing, setChanging] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function change(event: React.FormEvent) {
    event.preventDefault();
    setChanging(true);
    setProblem(null);

    try {
      await changeThreadStatus(threadId, next);
      router.refresh();
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "상태를 바꾸지 못했습니다.",
      );
    } finally {
      setChanging(false);
    }
  }

  return (
    <form onSubmit={change} className="mt-6 border-t border-line pt-5">
      <h2 className="text-label mb-2 font-bold tracking-[0.12em] text-faint uppercase">
        관리
      </h2>
      <FormActions>
        <label htmlFor="thread-status" className="text-note text-muted">
          상태
        </label>
        <select
          id="thread-status"
          value={next}
          onChange={(event) => setNext(event.target.value)}
          className={`${inputStyle} w-auto`}
        >
          {choices.map((choice) => (
            <option key={choice.value} value={choice.value}>
              {choice.label}
            </option>
          ))}
        </select>
        <button type="submit" disabled={changing} className={buttonStyle()}>
          {changing ? "바꾸는 중" : "상태 바꾸기"}
        </button>
      </FormActions>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
