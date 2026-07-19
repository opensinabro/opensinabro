import type { CSSProperties } from "react";
import type { StyleDeclaration } from "@/lib/namumark/StyleDeclaration";

/** React의 style 객체는 camelCase만 받는다 — `background-color`는 조용히 무시된다. */
function camelCase(property: string): string {
  // 사용자 정의 속성(`--이름`)은 React가 그대로 넘기므로 손대지 않는다.
  if (property.startsWith("--")) {
    return property;
  }
  return property.replace(/-([a-z])/g, (_, letter: string) =>
    letter.toUpperCase(),
  );
}

/**
 * IR이 이미 걸러 낸 선언들을 React style 객체로 옮긴다.
 *
 * 값 검증은 여기서 하지 않는다 — 무엇을 받아들일지는 resolve가 정했고, 여기 온 선언은
 * 이미 그 판정을 지났다.
 */
export function styleObject(
  declarations: StyleDeclaration[],
): CSSProperties | undefined {
  if (declarations.length === 0) {
    return undefined;
  }
  const style: Record<string, string> = {};
  for (const { property, value } of declarations) {
    style[camelCase(property)] = value;
  }
  return style as CSSProperties;
}

/** 여러 조각을 이어 클래스 문자열을 만든다. 남는 게 없으면 속성 자체를 두지 않는다. */
export function classNames(
  ...parts: (string | false | null | undefined)[]
): string | undefined {
  const kept = parts.filter((part): part is string => Boolean(part));
  return kept.length > 0 ? kept.join(" ") : undefined;
}
