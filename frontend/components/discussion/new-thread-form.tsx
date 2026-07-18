"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/layout/notice";
import { buttonStyle } from "@/components/ui/button";
import { Field, FormActions, FormLayout, inputStyle } from "@/components/ui/field";
import { createThread } from "@/lib/api/discussion-client";
import { wikiPath } from "@/lib/wiki-path";

export function NewThreadForm({ title }: { title: string }) {
  const router = useRouter();
  const [topic, setTopic] = useState("");
  const [content, setContent] = useState("");
  const [opening, setOpening] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  async function open(event: React.FormEvent) {
    event.preventDefault();
    setOpening(true);
    setProblem(null);

    try {
      const threadId = await createThread(title, { topic, content });
      router.push(wikiPath.discussThread(threadId));
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "토론을 열지 못했습니다.",
      );
      setOpening(false);
    }
  }

  return (
    <form onSubmit={open} className="mt-6 border-t border-line pt-5">
      <FormLayout>
        <Field label="주제" htmlFor="thread-topic">
          <input
            id="thread-topic"
            value={topic}
            onChange={(event) => setTopic(event.target.value)}
            required
            className={inputStyle}
          />
        </Field>
        <Field label="내용" htmlFor="thread-content">
          <textarea
            id="thread-content"
            value={content}
            onChange={(event) => setContent(event.target.value)}
            required
            rows={5}
            className={inputStyle}
          />
        </Field>
        <FormActions>
          <button
            type="submit"
            disabled={opening}
            className={buttonStyle({ tone: "primary" })}
          >
            {opening ? "여는 중" : "토론 열기"}
          </button>
        </FormActions>
      </FormLayout>
      {problem && <Alert tone="danger">{problem}</Alert>}
    </form>
  );
}
