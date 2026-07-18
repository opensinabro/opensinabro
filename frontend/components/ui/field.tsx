import { cloneElement, isValidElement } from "react";

// 폼은 화면마다 필드 몇 개와 단추 하나라는 같은 모양이다. 라벨 위치·간격·오류 표시를
// 여기 모아 두어야 로그인·이동·삭제·올리기가 서로 다른 폼처럼 보이지 않는다.

export function Field({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}) {
  // 곁말을 그리기만 하면 눈으로 읽는 사람에게만 닿는다. htmlFor를 필수로 받아
  // 라벨을 이미 묶고 있으므로, 같은 값으로 곁말도 입력에 묶는다.
  const hintId = hint ? `${htmlFor}-hint` : undefined;
  const described =
    hintId && isValidElement<{ "aria-describedby"?: string }>(children)
      ? cloneElement(children, { "aria-describedby": hintId })
      : children;

  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={htmlFor} className="text-note font-semibold text-ink">
        {label}
      </label>
      {described}
      {hint && (
        <p id={hintId} className="text-fine m-0 text-faint">
          {hint}
        </p>
      )}
    </div>
  );
}

export const inputStyle =
  "text-note w-full rounded border border-line bg-ground px-2.5 py-1.5 text-body focus-visible:border-accent focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent";

export function FormLayout({ children }: { children: React.ReactNode }) {
  return <div className="flex max-w-[420px] flex-col gap-3.5">{children}</div>;
}

export function FormActions({ children }: { children: React.ReactNode }) {
  return <div className="mt-1 flex items-center gap-2">{children}</div>;
}
