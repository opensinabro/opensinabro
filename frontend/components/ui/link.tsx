import { cva } from "class-variance-authority";

// 위키 안을 가리키는 링크는 어디에 있든 같은 모양이다. 문자열을 라우트마다 적으면
// 밑줄이나 색이 화면별로 갈리므로 여기서만 정한다 — Link에도 button에도 붙는다.
export const linkStyle = cva("text-link hover:underline", {
  variants: {
    /** 둘러싼 글의 크기를 따르지 않고 링크가 직접 정해야 할 때만 쓴다. */
    size: { inherit: "", ui: "text-ui", note: "text-note" },
    weight: { normal: "", medium: "font-medium" },
  },
  defaultVariants: { size: "inherit", weight: "normal" },
});
