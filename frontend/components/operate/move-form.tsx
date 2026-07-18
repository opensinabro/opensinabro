"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import {
  Field,
  FormActions,
  FormLayout,
  inputStyle,
} from "@/components/ui/field";
import { moveDocument } from "@/lib/api/operate-client";
import { wikiPath } from "@/lib/wiki-path";

export function MoveForm({ title }: { title: string }) {
  const router = useRouter();
  const [target, setTarget] = useState(title);
  const [comment, setComment] = useState("");
  const [moving, setMoving] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function move(event: React.FormEvent) {
    event.preventDefault();
    setMoving(true);
    setProblem(null);

    try {
      const moved = await moveDocument(title, { target, comment });
      router.push(wikiPath.read(moved));
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "문서를 옮기지 못했습니다.",
      );
      setMoving(false);
    }
  }

  return (
    <form onSubmit={move} className="mt-5">
      <FormLayout>
        <Field label="도착 제목" htmlFor="move-target">
          <input
            id="move-target"
            value={target}
            onChange={(event) => setTarget(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field
          label="사유"
          htmlFor="move-comment"
          hint="다섯 자 이상 적어 주세요."
        >
          <input
            id="move-comment"
            value={comment}
            onChange={(event) => setComment(event.target.value)}
            required
            minLength={5}
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={moving}
            className={buttonStyle({ tone: "primary" })}
          >
            {moving ? "옮기는 중" : "옮기기"}
          </button>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
