import { cva } from "class-variance-authority";

// 단추와 단추처럼 보이는 링크가 같은 모양을 쓰도록 스타일만 따로 낸다 —
// Link에는 className으로, button에는 그대로 붙인다.
export const buttonStyle = cva(
  "text-ui inline-flex items-center rounded border px-3 py-1 disabled:opacity-60",
  {
    variants: {
      tone: {
        quiet: "border-line text-body hover:border-accent hover:text-accent-deep",
        primary: "border-accent bg-accent font-semibold text-white",
      },
    },
    defaultVariants: { tone: "quiet" },
  },
);
