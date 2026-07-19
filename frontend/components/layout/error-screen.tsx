import { Fragment } from "react";
import { Link } from "@/components/layout/link";
import { PageHeader } from "@/components/layout/page-header";
import { linkStyle } from "@/components/ui/link";

// 오류는 종류가 넷(404·403·401·500)인데 자리와 어법은 하나다. 화면마다 마크업을 짜면
// "권한이 없습니다"와 "찾을 수 없습니다"가 서로 다른 크기·다른 여백으로 선다.
//
// 위키에서 오류는 막다른 길이 아니라 문서 한 장이다 — 제목이 무슨 일인지 말하고,
// 부제가 어느 대상에서 났는지 밝히고, 마지막 줄이 여기서 갈 수 있는 곳을 준다.
// 갈 곳 없는 오류 화면은 사용자를 뒤로 가기 단추로 내몬다.

export type ErrorAction = { label: string; href: string };

/** 사용자가 손쓸 수 없는 장애에서만 곁들이는 것 — 신고할 때 옮겨 적을 값들. */
export type ErrorDiagnostic = { label: string; value: string };

export function ErrorScreen({
  title,
  subject,
  description,
  diagnostics,
  actions,
  onRetry,
}: {
  title: string;
  /** 오류가 난 대상 — 문서 제목이나 주소. */
  subject?: string;
  description: string;
  diagnostics?: ErrorDiagnostic[];
  actions?: ErrorAction[];
  /** 오류 경계에서만 넘어온다. 서버가 그리는 화면에는 되돌릴 상태가 없다. */
  onRetry?: () => void;
}) {
  const hasWayOut = Boolean(onRetry) || (actions?.length ?? 0) > 0;

  return (
    <>
      <PageHeader title={title} note={subject} />

      <div className="pt-5">
        <p className="text-note m-0 py-2 text-muted">{description}</p>

        {diagnostics && diagnostics.length > 0 && (
          // 값은 사용자가 그대로 옮겨 적을 것이므로 고정폭으로 낸다 — 비례폭에서는
          // 추적 번호의 0과 O가 갈리지 않는다.
          <dl className="text-fine mt-4 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1.5 border-t border-line-soft pt-3">
            {diagnostics.map((entry) => (
              <Fragment key={entry.label}>
                <dt className="text-faint">{entry.label}</dt>
                <dd className="m-0 font-mono break-all text-body">
                  {entry.value}
                </dd>
              </Fragment>
            ))}
          </dl>
        )}

        {hasWayOut && (
          <p className="text-note mt-4 mb-0 text-muted">
            할 수 있는 일{" "}
            {onRetry && (
              <button type="button" onClick={onRetry} className={linkStyle()}>
                다시 시도
              </button>
            )}
            {actions?.map((action, index) => (
              <Fragment key={action.href}>
                {(index > 0 || onRetry) && " · "}
                <Link href={action.href} className={linkStyle()}>
                  {action.label}
                </Link>
              </Fragment>
            ))}
          </p>
        )}
      </div>
    </>
  );
}
