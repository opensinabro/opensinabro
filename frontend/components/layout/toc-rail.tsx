"use client";

import { useEffect, useRef, useState } from "react";

export type TableOfContentsRailEntry = {
  /** "1.2.3" — 문단 앵커는 `s-{number}`다. */
  number: string;
  depth: number;
  text: string;
};

/** 점과 라벨이 함께 서는 칸의 높이. 이보다 촘촘한 문단은 아래로 밀린다. */
const slotHeight = 26;

// 문단이 놓인 비율이 곧 칸의 자리다. 다만 앞 칸과 겹치면 아래로 밀려난다 — 점과 라벨이
// 한 줄로 붙어 다니므로, 점만 제자리에 두면 짝이 어긋나 어느 라벨이 어느 점의 것인지
// 읽히지 않는다.
function slotTops(ratios: number[], trackHeight: number) {
  return ratios.reduce<number[]>((tops, ratio) => {
    const previous = tops.at(-1) ?? -Infinity;
    tops.push(
      Math.max(ratio * trackHeight - slotHeight / 2, previous + slotHeight),
    );
    return tops;
  }, []);
}

export function TableOfContentsRail({
  entries,
}: {
  entries: TableOfContentsRailEntry[];
}) {
  const [activeNumber, setActiveNumber] = useState<string | null>(null);
  // 점을 직접 겨눈 것과 라벨 위에 손이 놓인 것은 다른 사건이다. 앞은 목록을 그 하나로
  // 좁히고, 뒤는 이미 펼쳐진 목록에서 누를 것을 짚어 보일 뿐이다.
  const [pointedNumber, setPointedNumber] = useState<string | null>(null);
  const [hoveredNumber, setHoveredNumber] = useState<string | null>(null);
  const [opened, setOpened] = useState(false);
  // 점의 자리는 스크롤 진행이 아니라 문서 안에서 그 문단이 놓인 비율이다 — 축이 곧
  // 문서의 축소판이라, 점 사이 간격이 문단의 길이를 그대로 나른다.
  const [ratios, setRatios] = useState<number[]>([]);
  const [copied, setCopied] = useState(false);
  const track = useRef<HTMLDivElement>(null);
  const [trackHeight, setTrackHeight] = useState(0);

  useEffect(() => {
    let queued = false;

    const measure = () => {
      queued = false;

      const total = Math.max(document.documentElement.scrollHeight, 1);
      setTrackHeight(track.current?.clientHeight ?? 0);

      setRatios(
        entries.map((entry) => {
          const section = document.getElementById(`s-${entry.number}`);
          if (section === null) return 0;

          const top = section.getBoundingClientRect().top + window.scrollY;
          return Math.min(1, top / total);
        }),
      );

      // 화면 위쪽 기준선을 막 지난 문단이 "지금 읽는 곳"이다. 관찰자를 쓰면 문단이
      // 화면보다 길 때 아무것도 교차하지 않는 구간이 생긴다.
      const passed = entries.filter((entry) => {
        const section = document.getElementById(`s-${entry.number}`);
        return section !== null && section.getBoundingClientRect().top <= 120;
      });

      setActiveNumber(passed.at(-1)?.number ?? null);
    };

    const onScroll = () => {
      if (queued) return;
      queued = true;
      requestAnimationFrame(measure);
    };

    measure();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);

    return () => {
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  }, [entries]);

  const tops = slotTops(ratios, trackHeight);

  return (
    <div className="pointer-events-none fixed inset-y-0 right-0 z-10 hidden w-44 rail:block">
      <nav
        aria-label="목차"
        className="pointer-events-auto absolute inset-y-16 right-0 w-44"
        onMouseEnter={() => setOpened(true)}
        onMouseLeave={() => {
          setOpened(false);
          setPointedNumber(null);
          setHoveredNumber(null);
        }}
        onFocus={() => setOpened(true)}
        onBlur={() => setOpened(false)}
      >
        <div ref={track} className="relative h-full">
          {entries.map((entry, index) => {
            const active = entry.number === activeNumber;

            return (
              <a
                key={entry.number}
                href={`#s-${entry.number}`}
                aria-current={active ? "location" : undefined}
                // 점 하나가 곧 과녁이다. 점은 6px이지만 누를 자리는 그보다 넓어야
                // 하므로, 링크가 축 왼쪽까지 뻗고 그 안에서 점만 오른쪽 끝에 선다.
                className="group absolute right-0 flex w-16 items-center justify-end pr-3 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
                style={{ top: tops[index] ?? 0, height: slotHeight }}
                onMouseEnter={() => setPointedNumber(entry.number)}
                onMouseLeave={() => setPointedNumber(null)}
                onFocus={() => setPointedNumber(entry.number)}
              >
                <span className="sr-only">
                  {entry.number}. {entry.text}
                </span>

                {/* 깊이는 점의 크기로 나르지 않는다 — 커지는 점은 지금 읽는 문단과
                    손이 닿은 문단뿐이라, 축을 흘긋 볼 때 찾을 것이 하나로 좁혀진다. */}
                <span
                  aria-hidden="true"
                  className={`shrink-0 rounded-full bg-ink transition-[width,height] group-hover:size-2 ${
                    active ? "size-2" : "size-1.5"
                  }`}
                />
              </a>
            );
          })}

          {/* 라벨은 제 점과 같은 칸에서 가운데를 맞춘다. 축 근처에 손이 오면 전부 뜨고,
              점 하나를 정확히 겨누면 그 하나만 남는다 — 축을 훑을 때는 목차 전체를
              읽을 수 있어야 하고, 겨눈 뒤에는 무엇을 누르는지가 분명해야 한다. */}
          <div
            aria-hidden="true"
            // 겹쳐 깔린 판이 점의 hover를 삼키면 안 된다 — 손을 받는 것은 라벨 자신뿐이다.
            className={`pointer-events-none absolute inset-0 transition-opacity ${
              opened ? "opacity-100" : "opacity-0"
            }`}
          >
            {entries.map((entry, index) => {
              const pointed = entry.number === pointedNumber;
              const hovered = entry.number === hoveredNumber || pointed;
              const active = entry.number === activeNumber;

              return (
                <a
                  key={entry.number}
                  href={`#s-${entry.number}`}
                  tabIndex={-1}
                  className={`text-fine absolute right-6 flex max-w-32 items-center gap-1 rounded border border-ink px-1.5 whitespace-nowrap text-ink transition-opacity ${
                    !opened || (pointedNumber !== null && !pointed)
                      ? "opacity-0"
                      : "pointer-events-auto opacity-100"
                  } ${hovered ? "bg-ground-sub" : "bg-ground"} ${
                    active ? "font-bold" : ""
                  }`}
                  style={{ top: tops[index] ?? 0, height: slotHeight }}
                  onMouseEnter={() => setHoveredNumber(entry.number)}
                  onMouseLeave={() => setHoveredNumber(null)}
                >
                  <span className="font-mono tabular-nums">{entry.number}.</span>
                  <span className="truncate">{entry.text}</span>
                </a>
              );
            })}
          </div>

          <div className="text-fine absolute right-3 bottom-0 flex flex-col items-end gap-1 text-ink">
            <button
              type="button"
              onClick={() => window.scrollTo({ top: 0 })}
              className="rounded hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
            >
              ↑ 맨 위로
            </button>

            <button
              type="button"
              onClick={() => {
                void navigator.clipboard.writeText(window.location.href).then(() => {
                  setCopied(true);
                  window.setTimeout(() => setCopied(false), 1600);
                });
              }}
              className="rounded hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
            >
              {copied ? "복사했습니다" : "⧉ 링크 복사"}
            </button>
          </div>
        </div>
      </nav>
    </div>
  );
}
