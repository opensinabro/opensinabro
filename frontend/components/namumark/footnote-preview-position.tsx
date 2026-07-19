"use client";

import { autoUpdate, computePosition, flip, offset, shift } from "@floating-ui/dom";
import { useEffect } from "react";

const MARKER = ".wiki-fn";
const PREVIEW = ".wiki-fn-preview";
/** 표기와 상자 사이의 틈. 여기가 비면 상자로 손을 옮기는 사이에 닫힌다. */
const GAP = 8;
/** 화면 가장자리에 남기는 여백. */
const EDGE = 8;

/**
 * 열린 각주 미리보기를 화면 안에 세운다.
 *
 * 여닫기는 CSS(`:hover`·`:focus-within`)가 그대로 맡고, 이 컴포넌트는 열린 상자의
 * 자리만 다시 잡는다 — 스크립트가 닿기 전에도, 끝내 닿지 않아도 미리보기는 열린다.
 *
 * 본문에는 손대지 않는다. 문서 전체에 위임 청취자 하나만 두므로 각주 표기는 서버가
 * 그린 마크업 그대로다.
 */
export function FootnotePreviewPosition(): null {
  useEffect(() => {
    let release: (() => void) | undefined;

    const close = () => {
      release?.();
      release = undefined;
    };

    const open = (marker: Element) => {
      const preview = marker.querySelector<HTMLElement>(PREVIEW);
      if (preview === null) return;
      close();
      release = autoUpdate(marker, preview, () => {
        void computePosition(marker, preview, {
          strategy: "fixed",
          placement: "top",
          middleware: [offset(GAP), flip(), shift({ padding: EDGE })],
        }).then(({ x, y }) => {
          // 표 안에서 쓰던 자리(오른쪽 맞춤·가운데 여백·끌어올림)를 함께 지운다 —
          // 남겨 두면 여기서 잡은 좌표와 겹쳐 상자가 엉뚱한 데로 간다.
          Object.assign(preview.style, {
            position: "fixed",
            left: `${x}px`,
            top: `${y}px`,
            right: "auto",
            bottom: "auto",
            margin: "0",
            transform: "none",
          });
        });
      });
    };

    const enter = (event: Event) => {
      const marker = (event.target as Element | null)?.closest?.(MARKER);
      if (marker != null) open(marker);
    };

    const leave = (event: Event) => {
      if ((event.target as Element | null)?.closest?.(MARKER) != null) close();
    };

    document.addEventListener("pointerover", enter);
    document.addEventListener("pointerout", leave);
    document.addEventListener("focusin", enter);
    document.addEventListener("focusout", leave);
    return () => {
      close();
      document.removeEventListener("pointerover", enter);
      document.removeEventListener("pointerout", leave);
      document.removeEventListener("focusin", enter);
      document.removeEventListener("focusout", leave);
    };
  }, []);

  return null;
}
