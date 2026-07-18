import { formatMoment } from "@/lib/format";
import type { ThreadComment } from "@/lib/api/discussion";

// 관리 조작은 발언과 한 타임라인에 섞여 있고, 서버는 바뀐 값 하나만 준다.
// 문장 만들기를 화면이 맡으므로 종류가 늘면 여기만 손댄다.
function sentence(comment: ThreadComment) {
  const detail = comment.detail ?? "";
  switch (comment.kind) {
    case "status_change":
      return `상태를 ${detail}(으)로 바꿨습니다.`;
    case "topic_change":
      return `주제를 "${detail}"(으)로 바꿨습니다.`;
    case "document_move":
      return `토론을 ${detail} 문서로 옮겼습니다.`;
    default:
      return comment.hidden ? "(가려진 발언)" : comment.content;
  }
}

export function CommentTimeline({ comments }: { comments: ThreadComment[] }) {
  return (
    <ol className="m-0 list-none p-0">
      {comments.map((comment) => (
        <li
          key={comment.sequence}
          id={String(comment.sequence)}
          className="border-b border-line-soft py-2.5"
        >
          <div className="text-fine flex flex-wrap items-baseline gap-x-2 text-faint">
            <span className="tabular-nums">#{comment.sequence}</span>
            <span className="text-muted">{comment.author}</span>
            {comment.adminMarked && (
              <span className="rounded bg-accent-wash px-1.5 py-0.5 text-label font-semibold text-accent-deep">
                관리자
              </span>
            )}
            <span>{formatMoment(comment.createdAt)}</span>
          </div>
          <p
            className={`text-list mt-1 mb-0 whitespace-pre-wrap ${
              comment.kind === "comment" && !comment.hidden
                ? "text-body"
                : "text-muted"
            }`}
          >
            {sentence(comment)}
          </p>
        </li>
      ))}
    </ol>
  );
}
