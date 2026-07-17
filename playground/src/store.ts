import { create } from 'zustand'
import {
  ensureReady,
  listBackends,
  renderMarkup,
  type BackendInfo,
  type RenderResult,
} from '@/lib/wasm'

const SAMPLE = `= 나무마크 플레이그라운드 =
왼쪽에 '''나무마크'''를 입력하면 오른쪽에 실시간으로 렌더됩니다.

== 문법 맛보기 ==
 * 리스트 항목
 * ''기울임''과 __밑줄__, ~~취소선~~
 * {{{#!wiki style="color:red"
   색이 있는 상자}}}

[[문서 링크]]와 [[https://namu.wiki|바깥 링크]], 각주도 됩니다.[* 이렇게요.]

|| 표 || 헤더 ||
|| 셀 || 셀 ||
`

interface PlaygroundState {
  ready: boolean
  backends: BackendInfo[]
  backendId: string
  source: string
  output: RenderResult
  error: string | null
  init: () => Promise<void>
  setSource: (source: string) => void
  setBackendId: (backendId: string) => void
  resetSample: () => void
  render: () => void
}

export const usePlaygroundStore = create<PlaygroundState>((set, get) => ({
  ready: false,
  backends: [],
  backendId: 'namuwiki',
  source: SAMPLE,
  output: { html: '', css: '' },
  error: null,

  init: async () => {
    await ensureReady()
    set({ ready: true, backends: listBackends() })
    get().render()
  },

  setSource: (source) => set({ source }),
  setBackendId: (backendId) => set({ backendId }),
  resetSample: () => set({ source: SAMPLE }),

  render: () => {
    if (!get().ready) return
    try {
      set({ output: renderMarkup(get().source, get().backendId), error: null })
    } catch (cause) {
      set({ error: String(cause) })
    }
  },
}))
