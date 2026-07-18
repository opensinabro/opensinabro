"use client";

import { create } from "zustand";
import { persist } from "zustand/middleware";

// 저장하지 못한 원문을 브라우저에 남겨 둔다. 서버에 없는 상태이므로 Query가 아니라
// Zustand가 맡는다 (docs/design/07의 상태 계층).
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
    { name: "opensinabro-editor" },
  ),
);
