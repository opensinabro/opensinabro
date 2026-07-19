"use client";

import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { Alert } from "@/components/layout/notice";
import { RenderTree } from "@/components/namumark/render-tree";
import { buttonStyle } from "@/components/ui/button";
import { renderPreview, saveEdit } from "@/lib/api/client";
import { useDraftStore } from "@/lib/draft-store";
import { formatBytes } from "@/lib/format";
import { wikiPath } from "@/lib/wiki-path";
import type { EditView } from "@/lib/api/types";
import { linkStyle } from "@/components/ui/link";

// 타이핑마다 렌더를 부르지 않도록 잠시 멎은 뒤의 값만 흘려보낸다.
function useSettled<T>(value: T, delay = 500) {
  const [settled, setSettled] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);

  return settled;
}

function PaneLabel({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-label font-bold tracking-[0.12em] text-faint uppercase">
      {children}
    </span>
  );
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

  // 저장해 둔 초안과 미리보기 상태는 마운트 뒤에 복원된다 (lib/draft-store.ts).
  useEffect(() => {
    void useDraftStore.persist.rehydrate();
  }, []);

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
        router.push(wikiPath.read(document.title));
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
      setProblem(
        error instanceof Error ? error.message : "저장하지 못했습니다.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <div className="px-4 empty:hidden sm:px-6">
        {draft && draft !== document.content && !dirty && (
          <Alert>
            저장하지 않은 편집이 남아 있습니다.{" "}
            <button
              type="button"
              onClick={() => setContent(draft)}
              className={linkStyle()}
            >
              이어서 쓰기
            </button>
          </Alert>
        )}

        {conflict && (
          <Alert tone="danger">
            편집하는 사이에 다른 사람이 문서를 고쳤고, 같은 자리가 겹쳐 자동으로
            합치지 못했습니다. 충돌 표시(
            <code>&lt;&lt;&lt;&lt;&lt;&lt;&lt;</code>)를 정리한 뒤 다시 저장하세요.
          </Alert>
        )}

        {problem && <Alert tone="danger">{problem}</Alert>}
      </div>

      <div
        className={`grid min-h-0 flex-1 ${previewOpen ? "lg:grid-cols-2" : "grid-cols-1"}`}
      >
        <div className="flex min-w-0 flex-col border-line lg:border-r">
          <label htmlFor="source" className="px-4 pt-3.5 pb-1.5 sm:px-6">
            <PaneLabel>원문</PaneLabel>
          </label>
          <textarea
            id="source"
            value={content}
            onChange={(event) => setContent(event.target.value)}
            spellCheck={false}
            className="min-h-[320px] flex-1 resize-none border-0 px-4 pb-5 font-mono text-note leading-[1.8] text-body focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent sm:min-h-[460px] sm:px-6"
          />
        </div>

        {previewOpen && (
          <div className="flex min-w-0 flex-col bg-ground-sub">
            <div className="flex items-baseline justify-between px-4 pt-3.5 pb-1.5 sm:px-6">
              <PaneLabel>미리보기</PaneLabel>
              {preview.isFetching && (
                <span className="text-fine text-accent-deep">그리는 중</span>
              )}
            </div>
            {preview.isError ? (
              <p className="text-note px-4 text-muted sm:px-6">
                미리보기를 그리지 못했습니다.
              </p>
            ) : (
              <div className="px-4 pb-5 sm:px-6">
                {preview.data && <RenderTree tree={preview.data} />}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 저장은 편집 요약 바로 옆에 둔다 — 요약을 적는 자리와 확정하는 자리가 같다. */}
      <div className="flex flex-wrap items-center gap-2.5 border-t border-line bg-ground-sub px-4 py-2.5 sm:px-6">
        <label htmlFor="comment" className="text-note text-muted">
          편집 요약
        </label>
        <input
          id="comment"
          value={comment}
          onChange={(event) => setComment(event.target.value)}
          className="text-note min-w-0 flex-1 rounded border border-line bg-ground px-2.5 py-1 text-body focus:border-accent focus:outline-none"
        />
        <span className="text-fine tabular-nums text-faint">
          {formatBytes(new Blob([content]).size)}
        </span>
        <button
          type="button"
          onClick={togglePreview}
          aria-pressed={previewOpen}
          className={buttonStyle()}
        >
          미리보기 {previewOpen ? "끄기" : "켜기"}
        </button>
        <button
          type="button"
          onClick={save}
          disabled={saving}
          className={buttonStyle({ tone: "primary" })}
        >
          {saving ? "저장 중" : "저장"}
        </button>
      </div>
    </>
  );
}
