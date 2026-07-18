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
import { deleteDocument } from "@/lib/api/operate-client";
import { wikiPath } from "@/lib/wiki-path";

export function DeleteForm({ title }: { title: string }) {
  const router = useRouter();
  const [comment, setComment] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function remove(event: React.FormEvent) {
    event.preventDefault();
    setDeleting(true);
    setProblem(null);

    try {
      const deleted = await deleteDocument(title, comment);
      router.push(wikiPath.read(deleted));
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "문서를 지우지 못했습니다.",
      );
      setDeleting(false);
    }
  }

  return (
    <form onSubmit={remove} className="mt-5">
      <FormLayout>
        <Field
          label="사유"
          htmlFor="delete-comment"
          hint="다섯 자 이상 적어 주세요."
        >
          <input
            id="delete-comment"
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
            disabled={deleting}
            className={buttonStyle({ tone: "primary" })}
          >
            {deleting ? "지우는 중" : "지우기"}
          </button>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
