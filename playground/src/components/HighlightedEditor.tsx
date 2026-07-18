import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'

import { nodeLabel, scopeClass } from '@/lib/scopes'
import { themeCss } from '@/lib/themes'
import { cn } from '@/lib/utils'
import { inspectTokens, type Token } from '@/lib/wasm'
import { usePlaygroundStore } from '@/store'

/** 라인넘버 거터 너비(px)와 거터~본문 사이 여백(px). */
const GUTTER_WIDTH = 44
const GUTTER_GAP = 12

/** 두 레이어(백드롭·textarea)가 정확히 겹치도록 공유하는 메트릭. */
const LAYER: CSSProperties = {
  margin: 0,
  padding: `12px 16px 12px ${GUTTER_WIDTH + GUTTER_GAP}px`,
  border: 0,
  fontFamily: 'ui-monospace, "SF Mono", "JetBrains Mono", Menlo, monospace',
  fontSize: '14px',
  lineHeight: 1.6,
  letterSpacing: 'normal',
  tabSize: 2,
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  overflowWrap: 'break-word',
}

interface Segment {
  text: string
  className: string
  tokenIndex: number
}

/**
 * 토큰 스트림을 논리 줄 단위로 나눈다. 각 토큰 text의 개행마다 줄을 끊고,
 * 조각은 원래 토큰 인덱스를 지녀 hover 강조·앵커 탐색에 쓰인다. 줄바꿈(wrap)이
 * 나도 한 논리 줄은 하나의 블록이라 라인넘버가 첫 행에 고정된다.
 */
function buildLines(tokens: Token[] | null, source: string): Segment[][] {
  if (!tokens) {
    return source.split('\n').map((line) => (line ? [{ text: line, className: '', tokenIndex: -1 }] : []))
  }
  const lines: Segment[][] = [[]]
  tokens.forEach((token, tokenIndex) => {
    const className = scopeClass(token)
    const parts = token.text.split('\n')
    parts.forEach((part, partIndex) => {
      if (partIndex > 0) lines.push([])
      if (part) lines[lines.length - 1].push({ text: part, className, tokenIndex })
    })
  })
  return lines
}

/** 테마 CSS를 문서에 한 번만 주입한다. */
function useThemeStyles() {
  useEffect(() => {
    const id = 'nm-hl-style'
    if (document.getElementById(id)) return
    const element = document.createElement('style')
    element.id = id
    element.textContent = themeCss()
    document.head.appendChild(element)
  }, [])
}

/** 토큰 시작 오프셋(UTF-16 코드유닛) 누적. */
function cumulativeOffsets(tokens: Token[]): number[] {
  const offsets = new Array<number>(tokens.length)
  let accumulated = 0
  for (let index = 0; index < tokens.length; index += 1) {
    offsets[index] = accumulated
    accumulated += tokens[index].text.length
  }
  return offsets
}

/** 문자 오프셋을 담는 토큰 인덱스를 이분 탐색한다. */
function tokenIndexAt(offsets: number[], offset: number): number {
  let low = 0
  let high = offsets.length - 1
  let answer = 0
  while (low <= high) {
    const mid = (low + high) >> 1
    if (offsets[mid] <= offset) {
      answer = mid
      low = mid + 1
    } else {
      high = mid - 1
    }
  }
  return answer
}

/**
 * 인덱스 토큰이 속한 노드의 토큰 범위 [시작, 끝]. 같은 parent가 이어지는 만큼
 * 묶어 노드 하나의 겉모습을 근사한다. 본문/문서 같은 큰 컨테이너는 한 토큰만.
 */
function nodeRange(tokens: Token[], index: number): [number, number] {
  const parent = tokens[index].parent
  if (parent === 'Paragraph' || parent === 'Document') return [index, index]
  let low = index
  let high = index
  while (low - 1 >= 0 && tokens[low - 1].parent === parent) low -= 1
  while (high + 1 < tokens.length && tokens[high + 1].parent === parent) high += 1
  return [low, high]
}

/** (x, y) 아래 문자 오프셋. textarea에 대한 caret 히트테스트는 브라우저별 편차가 있어 폴백을 둔다. */
function caretOffsetAt(x: number, y: number): number | null {
  const owner = document as Document & {
    caretPositionFromPoint?: (x: number, y: number) => { offset: number } | null
    caretRangeFromPoint?: (x: number, y: number) => Range | null
  }
  try {
    if (owner.caretPositionFromPoint) {
      const position = owner.caretPositionFromPoint(x, y)
      return position ? position.offset : null
    }
    if (owner.caretRangeFromPoint) {
      const range = owner.caretRangeFromPoint(x, y)
      return range ? range.startOffset : null
    }
  } catch {
    return null
  }
  return null
}

interface HoverState {
  /** 강조할 토큰 범위 [시작, 끝]. */
  range: [number, number]
  /** 노드 의미(한국어). */
  label: string
  /** 툴팁 고정 좌표(뷰포트 기준). */
  top: number
  left: number
  /** 위쪽에 놓을 자리가 없으면 아래로. */
  below: boolean
}

/**
 * 왼쪽 입력 칸. 투명 textarea(caret·선택·IME)를 색을 입힌 백드롭 위에 겹쳐,
 * 별도 탭 없이 인라인으로 구문을 하이라이트한다. 커서를 올리면 노드 전체를
 * 강조하고, 그 노드의 의미를 노드 위(자리가 없으면 아래)에 고정해 띄운다.
 */
export function HighlightedEditor() {
  useThemeStyles()

  const source = usePlaygroundStore((state) => state.source)
  const setSource = usePlaygroundStore((state) => state.setSource)
  const ready = usePlaygroundStore((state) => state.ready)
  const themeId = usePlaygroundStore((state) => state.highlightThemeId)

  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const backdropRef = useRef<HTMLPreElement>(null)
  const frameRef = useRef<number | null>(null)
  const [hover, setHover] = useState<HoverState | null>(null)
  const hoverRef = useRef<HoverState | null>(null)

  // 백드롭은 현재 원문과 항상 일치해야 하므로 여기서 직접(디바운스 없이) 파싱한다.
  const tokens = useMemo(() => {
    if (!ready) return null
    try {
      return inspectTokens(source)
    } catch {
      return null
    }
  }, [ready, source])

  const offsets = useMemo(() => (tokens ? cumulativeOffsets(tokens) : []), [tokens])
  const lines = useMemo(() => buildLines(tokens, source), [tokens, source])

  const clearHover = () => {
    if (hoverRef.current) {
      hoverRef.current = null
      setHover(null)
    }
  }

  // 원문이 바뀌면 인덱스가 어긋나므로 툴팁을 걷는다.
  useEffect(() => {
    clearHover()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tokens])

  const syncScroll = () => {
    if (textareaRef.current && backdropRef.current) {
      backdropRef.current.scrollTop = textareaRef.current.scrollTop
      backdropRef.current.scrollLeft = textareaRef.current.scrollLeft
    }
    clearHover()
  }

  const handleMove = (event: React.MouseEvent) => {
    if (!tokens) return
    const x = event.clientX
    const y = event.clientY
    if (frameRef.current !== null) return
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = null
      const offset = caretOffsetAt(x, y)
      if (offset === null) return clearHover()
      const index = tokenIndexAt(offsets, offset)
      const token = tokens[index]
      if (!token || token.kind === 'Newline' || token.parent === 'Document') return clearHover()
      const [low, high] = nodeRange(tokens, index)
      const current = hoverRef.current
      // 같은 노드 위에서는 위치를 유지한다 — 커서를 따라다니지 않게.
      if (current && current.range[0] === low && current.range[1] === high) return
      const anchor = backdropRef.current?.querySelector(`[data-token="${low}"]`) as HTMLElement | null
      if (!anchor) return clearHover()
      const rect = anchor.getBoundingClientRect()
      const below = rect.top < 44
      const next: HoverState = {
        range: [low, high],
        label: nodeLabel(token),
        top: below ? rect.bottom + 6 : rect.top - 6,
        left: rect.left,
        below,
      }
      hoverRef.current = next
      setHover(next)
    })
  }

  return (
    <div className="nm-hl relative h-full w-full overflow-hidden border-r bg-background" data-hltheme={themeId}>
      <div
        aria-hidden
        style={{
          position: 'absolute',
          top: 0,
          bottom: 0,
          left: 0,
          width: GUTTER_WIDTH,
          borderRight: '1px solid var(--border)',
          background: '#fbfbfa',
          pointerEvents: 'none',
        }}
      />
      <pre ref={backdropRef} aria-hidden style={{ ...LAYER, position: 'absolute', inset: 0, overflow: 'hidden', color: '#1b1c1e', pointerEvents: 'none' }}>
        {lines.map((segments, lineNumber) => (
          <div key={lineNumber} style={{ position: 'relative', minHeight: '1.6em' }}>
            <span
              style={{
                position: 'absolute',
                left: -(GUTTER_WIDTH + GUTTER_GAP),
                width: GUTTER_WIDTH,
                paddingRight: 8,
                textAlign: 'right',
                color: '#b6b8be',
                userSelect: 'none',
                fontVariantNumeric: 'tabular-nums',
              }}
            >
              {lineNumber + 1}
            </span>
            {segments.map((segment, index) => (
              <span
                key={index}
                data-token={segment.tokenIndex}
                className={cn(
                  segment.className,
                  hover &&
                    segment.tokenIndex >= hover.range[0] &&
                    segment.tokenIndex <= hover.range[1] &&
                    'nm-tok-hover',
                )}
              >
                {segment.text}
              </span>
            ))}
          </div>
        ))}
      </pre>
      <textarea
        ref={textareaRef}
        value={source}
        onChange={(event) => setSource(event.target.value)}
        onScroll={syncScroll}
        onMouseMove={handleMove}
        onMouseLeave={clearHover}
        spellCheck={false}
        style={{
          ...LAYER,
          position: 'absolute',
          inset: 0,
          width: '100%',
          height: '100%',
          resize: 'none',
          background: 'transparent',
          color: 'transparent',
          caretColor: '#1b1c1e',
          outline: 'none',
          overflow: 'auto',
        }}
      />
      {hover ? (
        <div
          className="pointer-events-none fixed z-50 rounded-md border bg-popover px-2.5 py-1 text-xs font-medium text-foreground shadow-md"
          style={{
            top: hover.top,
            left: hover.left,
            transform: hover.below ? undefined : 'translateY(-100%)',
          }}
        >
          {hover.label}
        </div>
      ) : null}
    </div>
  )
}
