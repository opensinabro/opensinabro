"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import { Field, FormActions, FormLayout, inputStyle } from "@/components/ui/field";
import { addComment } from "@/lib/api/discussion-client";

export function CommentForm({ threadId }: { threadId: string }) {
  const router = useRouter();
  const [content, setContent] = useState("");
  const [sending, setSending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function send(event: React.FormEvent) {
    event.preventDefault();
    setSending(true);
    setProblem(null);

    try {
      await addComment(threadId, content);
      setContent("");
      router.refresh();
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "발언을 남기지 못했습니다.",
      );
    } finally {
      setSending(false);
    }
  }

  return (
    <form onSubmit={send} className="mt-6 border-t border-line pt-5">
      <FormLayout>
        <Field label="발언" htmlFor="comment-content">
          <textarea
            id="comment-content"
            value={content}
            onChange={(event) => setContent(event.target.value)}
            required
            rows={4}
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={sending}
            className={buttonStyle({ tone: "primary" })}
          >
            {sending ? "남기는 중" : "남기기"}
          </button>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
