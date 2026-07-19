"use client";

import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { SyntaxKeys } from "@/components/document/syntax-keys";
import { Link } from "@/components/layout/link";
import { Alert } from "@/components/layout/notice";
import { RenderTree } from "@/components/namumark/render-tree";
import { buttonStyle } from "@/components/ui/button";
import { linkStyle } from "@/components/ui/link";
import { renderPreview, saveEdit } from "@/lib/api/client";
import { useDraftStore } from "@/lib/draft-store";
import { wikiPath } from "@/lib/wiki-path";
import type { EditView } from "@/lib/api/types";

// 타이핑마다 렌더를 부르지 않도록 잠시 멎은 뒤의 값만 흘려보낸다.
function useSettled<T>(value: T, delay = 500) {
  const [settled, setSettled] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);

  return settled;
}

/**
 * 문서 편집기.
 *
 * 셸을 쓰지 않고 뷰포트를 통째로 쓴다. 화면에 서는 것은 문서 이름·나가기·미리보기·편집
 * 요약·저장 다섯뿐이고, 나머지 자리는 전부 원문과 미리보기가 가져간다. 리비전 번호와
 * 문서 크기는 싣지 않는다 — 쓰는 동안 아무 결정에도 쓰이지 않는 값이라, 자리를 차지한
 * 만큼 원문에서 빼앗는다.
 *
 * 넓은 화면과 좁은 화면이 다른 것을 그린다.
 *
 * - 넓은 화면: 원문과 미리보기가 좌우로 서고 크롬은 없다. 조작은 아래 가운데 떠 있는
 *   알약과 왼쪽 위 표식에 모인다.
 * - 좁은 화면: 나란히 세울 폭이 없으므로 한 번에 하나만 보이고 단추로 갈아 낀다.
 *   자판이 올라오면 떠 있는 단추 대신 문법 줄이 그 자리를 받는다 — 자판이 세로의 절반을
 *   먹는 동안 원문에 한 픽셀이라도 더 남기는 편이 낫다.
 */
export function DocumentEditor({ document }: { document: EditView }) {
  const router = useRouter();
  const { drafts, previewOpen, keep, discard, togglePreview } = useDraftStore();

  const source = useRef<HTMLTextAreaElement>(null);
  const draft = drafts[document.title];
  const [content, setContent] = useState(document.content);
  const [comment, setComment] = useState("");
  const [baseRevision, setBaseRevision] = useState(document.baseRevision);
  const [conflict, setConflict] = useState(false);
  const [saving, setSaving] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  // 좁은 화면이 지금 무엇을 보이는가. `previewOpen`(넓은 화면의 두 열 여부)과 뜻이 달라
  // 따로 든다 — 미리보기를 켜 둔 채 저장한 사람이 다음에 편집을 열었을 때 원문이 아니라
  // 결과부터 마주하면 안 된다.
  const [pane, setPane] = useState<"source" | "preview">("source");
  // 커서가 원문 안에 있는가. 문법 줄을 세울지 정하는 값이다.
  //
  // 자판이 떠 있는지는 묻지 않는다 — 알아낼 방법이 마땅치 않고(초점이 남은 채로 자판만
  // 닫히는 길이 iOS·Android 양쪽에 있다), 알아낸다 해도 쓸 데가 없다. 문법 줄은 자판이
  // 아니라 커서를 따라야 하고, 나머지 조작은 어느 쪽이든 늘 서 있어야 한다. 그래서
  // 자판 상태에 따라 사라지는 것을 화면에서 없앴다.
  const [editingSource, setEditingSource] = useState(false);
  // 좁은 화면에는 편집 요약을 세울 줄이 없다. 저장을 누른 뒤에 묻는다.
  const [askingSummary, setAskingSummary] = useState(false);

  // 저장해 둔 초안과 미리보기 상태는 마운트 뒤에 복원된다 (lib/draft-store.ts).
  useEffect(() => {
    void useDraftStore.persist.rehydrate();
  }, []);

  const settled = useSettled(content);
  const preview = useQuery({
    queryKey: ["preview", document.title, settled],
    queryFn: () => renderPreview(document.title, settled),
    enabled: previewOpen || pane === "preview",
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
        setAskingSummary(false);
        return;
      }

      setProblem("이 문서를 편집할 권한이 없습니다.");
      setAskingSummary(false);
    } catch (error) {
      setProblem(
        error instanceof Error ? error.message : "저장하지 못했습니다.",
      );
      setAskingSummary(false);
    } finally {
      setSaving(false);
    }
  }

  const showingPreview = pane === "preview";

  return (
    <main id="content" className="flex min-h-0 flex-1 flex-col">
      <div className="relative flex min-h-0 flex-1 lg:flex-row">
        <div
          className={`min-w-0 flex-1 flex-col border-line lg:flex lg:border-r ${
            showingPreview ? "hidden" : "flex"
          }`}
        >
          {/* 아래 여백은 떠 있는 조작이 마지막 줄을 가리지 않을 만큼 준다. 커서가 글
              끝에 닿아도 브라우저가 이 여백까지 밀어 올려 주므로 가려지지 않는다. */}
          <textarea
            ref={source}
            aria-label={`${document.title} 원문`}
            value={content}
            onChange={(event) => setContent(event.target.value)}
            onFocus={() => setEditingSource(true)}
            onBlur={() => setEditingSource(false)}
            spellCheck={false}
            className="text-note min-h-0 flex-1 resize-none border-0 px-4 pt-11 pb-20 font-mono leading-[1.8] text-body focus-visible:outline-none sm:px-6"
          />
        </div>

        <div
          className={`min-w-0 flex-1 overflow-auto bg-ground-sub ${
            showingPreview ? "block" : "hidden"
          } ${previewOpen ? "lg:block" : "lg:hidden"}`}
        >
          <div className="px-4 pt-11 pb-20 sm:px-6">
            {preview.isError ? (
              <p className="text-note m-0 text-muted">
                미리보기를 그리지 못했습니다.
              </p>
            ) : (
              preview.data && <RenderTree tree={preview.data} />
            )}
          </div>
        </div>

        {/* 편집 중에 알아야 하는 것은 어느 문서인가뿐이다. 표식은 원문 위에 얹혀 자리를
            빼앗지 않는다. */}
        <div className="absolute top-3 left-3 z-20 flex items-center gap-1.5 rounded-full border border-line-soft bg-ground/85 py-1 pr-3.5 pl-2 backdrop-blur sm:left-4">
          <Link
            href={wikiPath.read(document.title)}
            aria-label="편집 그만두고 문서로"
            className="text-ui rounded-full px-1.5 text-faint hover:bg-ground-deep hover:text-ink"
          >
            ←
          </Link>
          <span className="text-ui max-w-[42vw] truncate font-extrabold tracking-tight text-ink lg:max-w-[28vw]">
            {document.title}
          </span>
          {preview.isFetching && (
            <span className="text-fine hidden text-accent-deep sm:inline">
              그리는 중
            </span>
          )}
        </div>

        {/* 알림은 원문 쪽에 붙인다. 두 열 한가운데에 띄우면 경계에 걸쳐, 원문에 대한
            말인지 미리보기에 대한 말인지가 흐려진다. */}
        <div className="pointer-events-none absolute top-14 right-3 left-3 z-20 flex flex-col items-start gap-2 empty:hidden sm:right-4 sm:left-4">
          <div className="pointer-events-auto w-full max-w-[640px] empty:hidden">
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

            {/* 알림은 원문 위에 떠 있으므로 읽고 나면 치울 수 있어야 한다 — 크롬이
                없는 화면에서 닫히지 않는 상자는 원문 두 줄을 영영 가린다. */}
            {(conflict || problem) && (
              <Alert tone="danger">
                {conflict ? (
                  <>
                    편집하는 사이에 다른 사람이 문서를 고쳤고, 같은 자리가 겹쳐
                    자동으로 합치지 못했습니다. 충돌 표시(
                    <code>&lt;&lt;&lt;&lt;&lt;&lt;&lt;</code>)를 정리한 뒤 다시
                    저장하세요.
                  </>
                ) : (
                  problem
                )}{" "}
                <button
                  type="button"
                  onClick={() => {
                    setConflict(false);
                    setProblem(null);
                  }}
                  className={linkStyle()}
                >
                  닫기
                </button>
              </Alert>
            )}
          </div>
        </div>

        {/* 넓은 화면의 조작. 요약을 적는 자리와 확정하는 자리를 한 알약에 붙여 둔다. */}
        <div className="absolute bottom-5 left-1/2 z-20 hidden -translate-x-1/2 items-center gap-2 rounded-full border border-line bg-ground py-1.5 pr-1.5 pl-4 shadow-[0_16px_38px_-20px_#14201da6] lg:flex">
          <input
            aria-label="편집 요약"
            value={comment}
            onChange={(event) => setComment(event.target.value)}
            placeholder="편집 요약"
            className="text-note w-[240px] border-0 bg-transparent text-body placeholder:text-placeholder focus:outline-none"
          />
          <button
            type="button"
            onClick={togglePreview}
            aria-pressed={previewOpen}
            className={buttonStyle({ className: "rounded-full" })}
          >
            미리보기
          </button>
          <button
            type="button"
            onClick={save}
            disabled={saving}
            className={buttonStyle({
              tone: "primary",
              className: "rounded-full px-5",
            })}
          >
            {saving ? "저장 중" : "저장"}
          </button>
        </div>

        {/* 좁은 화면의 조작. 늘 서 있는다 — 무엇에 따라 사라지든 그 조건을 잘못
            읽는 순간 저장할 길이 없는 화면이 되고, 그때 사용자가 할 수 있는 일이 없다.
            문법 줄은 이 아래 칸에 따로 서므로 서로 겹치지 않는다. */}
        <button
          type="button"
          onClick={() => setPane(showingPreview ? "source" : "preview")}
          // 원문 위에 뜨므로 바탕을 스스로 깔아야 한다 — 공용 단추는 띠 위에 서는 것을
          // 전제해 배경이 없고, 그대로 두면 아래 글자가 단추를 뚫고 비친다.
          className={buttonStyle({
            className:
              "absolute bottom-4 left-3 z-20 rounded-full bg-ground px-5 py-2 shadow-[0_12px_28px_-18px_#14201d99] sm:left-4 lg:hidden",
          })}
        >
          {showingPreview ? "원문" : "미리보기"}
        </button>
        <button
          type="button"
          onClick={() => setAskingSummary(true)}
          className={buttonStyle({
            tone: "primary",
            className:
              "absolute right-3 bottom-4 z-20 rounded-full px-5 py-2 shadow-[0_12px_28px_-16px_#14201da6] sm:right-4 lg:hidden",
          })}
        >
          저장
        </button>
      </div>

      {editingSource && !showingPreview && <SyntaxKeys textarea={source} />}

      {askingSummary && (
        <div className="fixed inset-0 z-30 flex flex-col justify-end lg:hidden">
          <button
            type="button"
            aria-label="저장 그만두고 계속 쓰기"
            onClick={() => setAskingSummary(false)}
            className="flex-1 bg-ink/30"
          />
          <div className="flex flex-col gap-3 rounded-t-2xl border-t border-line bg-ground px-4 pt-4 pb-6 shadow-[0_-14px_34px_-22px_#14201d99]">
            <h2 className="text-brand m-0 font-extrabold tracking-tight text-ink">
              {document.title} 저장
            </h2>
            <input
              aria-label="편집 요약"
              value={comment}
              onChange={(event) => setComment(event.target.value)}
              placeholder="편집 요약 — 무엇을 고쳤는지 한 줄로"
              className="text-note rounded border border-line bg-ground px-3 py-2.5 text-body placeholder:text-placeholder focus:border-accent focus:outline-none"
            />
            <button
              type="button"
              onClick={save}
              disabled={saving}
              className={buttonStyle({
                tone: "primary",
                className: "justify-center py-2.5",
              })}
            >
              {saving ? "저장 중" : "저장"}
            </button>
            <button
              type="button"
              onClick={() => setAskingSummary(false)}
              className="text-ui rounded py-1 text-faint"
            >
              계속 쓰기
            </button>
          </div>
        </div>
      )}
    </main>
  );
}
