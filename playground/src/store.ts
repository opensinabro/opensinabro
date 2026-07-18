import { create } from 'zustand'
import {
  ensureReady,
  listBackends,
  renderMarkup,
  type BackendInfo,
  type RenderResult,
} from '@/lib/wasm'
import { EXAMPLES } from '@/examples'
import { THEMES } from '@/lib/themes'

const THEME_STORAGE_KEY = 'nm-hl-theme'

function initialThemeId(): string {
  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY)
    if (saved && THEMES.some((theme) => theme.id === saved)) return saved
  } catch {
    /* localStorage 접근 불가 시 기본값 */
  }
  return THEMES[0].id
}

/** 모바일에서는 편집기와 미리보기를 나란히 둘 폭이 없어 한 번에 하나만 보인다. */
export type MobilePane = 'editor' | 'preview'

interface PlaygroundState {
  ready: boolean
  backends: BackendInfo[]
  backendId: string
  exampleId: string
  source: string
  highlightThemeId: string
  mobilePane: MobilePane
  output: RenderResult
  error: string | null
  init: () => Promise<void>
  setSource: (source: string) => void
  setBackendId: (backendId: string) => void
  setHighlightTheme: (themeId: string) => void
  setMobilePane: (pane: MobilePane) => void
  loadExample: (exampleId: string) => void
  render: () => void
}

export const usePlaygroundStore = create<PlaygroundState>((set, get) => ({
  ready: false,
  backends: [],
  backendId: 'namuwiki',
  exampleId: EXAMPLES[0].id,
  source: EXAMPLES[0].source,
  highlightThemeId: initialThemeId(),
  mobilePane: 'editor',
  output: { html: '', css: '' },
  error: null,

  init: async () => {
    await ensureReady()
    set({ ready: true, backends: listBackends() })
    get().render()
  },

  setSource: (source) => set({ source }),
  setBackendId: (backendId) => set({ backendId }),
  setHighlightTheme: (themeId) => {
    set({ highlightThemeId: themeId })
    try {
      localStorage.setItem(THEME_STORAGE_KEY, themeId)
    } catch {
      /* 지속 실패는 무시 — 세션 내에서는 반영됨 */
    }
  },
  setMobilePane: (pane) => set({ mobilePane: pane }),
  loadExample: (exampleId) => {
    const example = EXAMPLES.find((item) => item.id === exampleId)
    if (example) set({ exampleId, source: example.source })
  },

  render: () => {
    if (!get().ready) return
    try {
      set({ output: renderMarkup(get().source, get().backendId), error: null })
    } catch (cause) {
      set({ error: String(cause) })
    }
  },
}))
