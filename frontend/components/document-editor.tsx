"use client";

import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { renderPreview, saveEdit } from "@/lib/client-api";
import { useDraftStore } from "@/lib/draft-store";
import type { EditView } from "@/lib/api";

// 타이핑마다 렌더를 부르지 않도록 잠시 멎은 뒤의 값만 흘려보낸다.
function useSettled<T>(value: T, delay = 500) {
  const [settled, setSettled] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);

  return settled;
}

export function DocumentEditor({ document }: { document: EditView }) {
  const router = useRouter();
  const { drafts, previewOpen, keep, discard, togglePreview } = useDraftStore();

  const draft = drafts[document.title];
  const [content, setContent] = useState(document.content);
  const [comment, setComment] = useState("");
  const [baseRevision, setBaseRevision] = useState(document.baseRevision);
  const [conflict, setConflict] = useState(false);
  const [saving, setSaving] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  const settled = useSettled(content);
  const preview = useQuery({
    queryKey: ["preview", document.title, settled],
    queryFn: () => renderPreview(document.title, settled),
    enabled: previewOpen,
  });

  const dirty = content !== document.content;

  useEffect(() => {
    if (dirty) keep(document.title, content);
  }, [content, dirty, document.title, keep]);

  // 문서에서 벗어날 때 저장하지 않은 편집이 있으면 알린다.
  useEffect(() => {
    if (!dirty) return;

    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);

  async function save() {
    setSaving(true);
    setProblem(null);

    try {
      const result = await saveEdit(document.title, {
        baseRevision,
        content,
        comment,
      });

      if (result.kind === "saved") {
        discard(document.title);
        router.push(`/w/${document.title}`);
        return;
      }

      if (result.kind === "conflict") {
        setContent(result.content);
        setBaseRevision(result.baseRevision);
        setConflict(true);
        return;
      }

      setProblem("이 문서를 편집할 권한이 없습니다.");
    } catch (error) {
      setProblem(error instanceof Error ? error.message : "저장하지 못했습니다.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex min-w-0 flex-col xl:col-span-2">
      <div className="flex flex-wrap items-center gap-2.5 px-6 pt-4">
        <h1 className="m-0 text-2xl font-extrabold tracking-tight text-ink">
          {document.title}
        </h1>
        <span className="rounded bg-ground-deep px-2 py-0.5 text-[11px] font-semibold text-muted">
          편집
        </span>
        <div className="ml-auto flex items-center gap-1.5">
          <button
            type="button"
            onClick={togglePreview}
            aria-pressed={previewOpen}
            className="rounded border border-line px-3 py-1 text-[13px] text-body hover:border-accent hover:text-accent-deep"
          >
            미리보기 {previewOpen ? "끄기" : "켜기"}
          </button>
          <button
            type="button"
            onClick={save}
            disabled={saving}
            className="rounded border border-accent bg-accent px-3 py-1 text-[13px] font-semibold text-white disabled:opacity-60"
          >
            {saving ? "저장 중" : "저장"}
          </button>
        </div>
      </div>
      <div className="mt-3 border-b border-line" />

      {draft && draft !== document.content && !dirty && (
        <p className="mx-6 mt-3 rounded border border-line bg-ground-sub px-3 py-2 text-[12.5px] text-muted">
          저장하지 않은 편집이 남아 있습니다.{" "}
          <button
            type="button"
            onClick={() => setContent(draft)}
            className="text-link hover:underline"
          >
            이어서 쓰기
          </button>
        </p>
      )}

      {conflict && (
        <p className="mx-6 mt-3 rounded border border-[#f2d3cd] bg-[#fdf1ef] px-3 py-2 text-[12.5px] text-[#9c3a2c]">
          편집하는 사이에 다른 사람이 문서를 고쳤고, 같은 자리가 겹쳐 자동으로 합치지
          못했습니다. 충돌 표시(<code>&lt;&lt;&lt;&lt;&lt;&lt;&lt;</code>)를 정리한 뒤
          다시 저장하세요.
        </p>
      )}

      {problem && (
        <p className="mx-6 mt-3 rounded border border-[#f2d3cd] bg-[#fdf1ef] px-3 py-2 text-[12.5px] text-[#9c3a2c]">
          {problem}
        </p>
      )}

      <div
        className={`grid min-h-0 flex-1 gap-0 ${previewOpen ? "lg:grid-cols-2" : "grid-cols-1"}`}
      >
        <div className="flex min-w-0 flex-col border-line lg:border-r">
          <label
            htmlFor="source"
            className="px-6 pt-3.5 pb-1.5 text-[11px] font-bold tracking-[0.12em] text-faint uppercase"
          >
            원문
          </label>
          <textarea
            id="source"
            value={content}
            onChange={(event) => setContent(event.target.value)}
            spellCheck={false}
            className="min-h-[460px] flex-1 resize-none border-0 px-6 pb-5 font-mono text-[12.5px] leading-[1.8] text-body focus:outline-none"
          />
        </div>

        {previewOpen && (
          <div className="flex min-w-0 flex-col bg-[#fcfdfc]">
            <div className="flex items-baseline justify-between px-6 pt-3.5 pb-1.5">
              <span className="text-[11px] font-bold tracking-[0.12em] text-faint uppercase">
                미리보기
              </span>
              {preview.isFetching && (
                <span className="text-[11.5px] text-accent-deep">그리는 중</span>
              )}
            </div>
            {preview.isError ? (
              <p className="px-6 text-[12.5px] text-muted">
                미리보기를 그리지 못했습니다.
              </p>
            ) : (
              <div
                className="wiki-content px-6 pb-5"
                dangerouslySetInnerHTML={{ __html: preview.data ?? "" }}
              />
            )}
          </div>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-2.5 border-t border-line bg-ground-sub px-6 py-2.5">
        <label htmlFor="comment" className="text-[12.5px] text-muted">
          편집 요약
        </label>
        <input
          id="comment"
          value={comment}
          onChange={(event) => setComment(event.target.value)}
          className="min-w-0 flex-1 rounded border border-line bg-ground px-2.5 py-1 text-[12.5px] text-body focus:border-accent focus:outline-none"
        />
        <span className="tabular-nums text-[12px] text-faint">
          {new Blob([content]).size.toLocaleString()} B
        </span>
      </div>
    </div>
  );
}
