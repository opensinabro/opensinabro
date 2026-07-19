import { create } from "zustand";

// 이동 중인 링크가 몇 개인지 센다. 불리언 하나로 두면 링크를 잇달아 눌렀을 때 먼저
// 끝난 쪽이 아직 기다리는 쪽의 표시까지 꺼 버린다.
type NavigationProgress = {
  waiting: number;
  begin: () => void;
  finish: () => void;
};

export const useNavigationProgress = create<NavigationProgress>((set) => ({
  waiting: 0,
  begin: () => set((state) => ({ waiting: state.waiting + 1 })),
  finish: () => set((state) => ({ waiting: Math.max(0, state.waiting - 1) })),
}));
