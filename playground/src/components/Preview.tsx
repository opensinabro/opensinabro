import { useEffect, useRef } from 'react'

import type { RenderResult } from '@/lib/wasm'

/**
 * 백엔드 계약상 기본 글자·배경색은 스킨의 몫이다. 플레이그라운드가 그 스킨 역할을
 * 맡아 프리뷰 뿌리에 기본색을 준다 — 백엔드 CSS는 컴포넌트만 칠한다.
 */
const BASE_STYLE = `
.wiki { color: #1c1e21; font-family: system-ui, "Apple SD Gothic Neo", sans-serif; line-height: 1.7; }
`

/**
 * 렌더 결과를 Shadow DOM 안에 격리해 그린다 — 백엔드 CSS가 앱(Tailwind) 스타일과
 * 섞이지 않고, 백엔드의 클래스 어휘(wiki-*)가 그대로 산다.
 */
export function Preview({ output }: { output: RenderResult }) {
  const hostRef = useRef<HTMLDivElement>(null)
  const rootRef = useRef<ShadowRoot | null>(null)

  useEffect(() => {
    if (!hostRef.current) return
    if (!rootRef.current) {
      rootRef.current = hostRef.current.attachShadow({ mode: 'open' })
    }
    rootRef.current.innerHTML = `<style>${BASE_STYLE}${output.css}</style><div class="wiki">${output.html}</div>`
  }, [output])

  return <div ref={hostRef} />
}
