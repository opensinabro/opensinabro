"use client";

import { create } from "zustand";
import { persist } from "zustand/middleware";

// 저장하지 못한 원문을 브라우저에 남겨 둔다. 서버에 없는 상태이므로 Query가 아니라
// Zustand가 맡는다 (docs/architecture.md의 프론트엔드 상태 계층).
type DraftStore = {
  drafts: Record<string, string>;
  previewOpen: boolean;
  keep: (title: string, content: string) => void;
  discard: (title: string) => void;
  togglePreview: () => void;
};

export const useDraftStore = create<DraftStore>()(
  persist(
    (set) => ({
      drafts: {},
      previewOpen: true,
      keep: (title, content) =>
        set((state) => ({ drafts: { ...state.drafts, [title]: content } })),
      discard: (title) =>
        set((state) => ({
          drafts: Object.fromEntries(
            Object.entries(state.drafts).filter(([kept]) => kept !== title),
          ),
        })),
      togglePreview: () => set((state) => ({ previewOpen: !state.previewOpen })),
    }),
    // 브라우저에만 있는 값이라 서버가 그린 첫 화면과 어긋난다(미리보기를 꺼 둔 사람은
    // 서버는 켠 상태로, 브라우저는 끈 상태로 그린다). 복원을 마운트 뒤로 미뤄
    // 하이드레이션이 어긋나지 않게 한다.
    { name: "opensinabro-editor", skipHydration: true },
  ),
);
