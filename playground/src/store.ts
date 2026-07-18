import { create } from 'zustand'
import {
  ensureReady,
  inspectTokens,
  listBackends,
  renderMarkup,
  type BackendInfo,
  type RenderResult,
  type Token,
} from '@/lib/wasm'
import { EXAMPLES } from '@/examples'

export type ViewMode = 'preview' | 'tokens'

interface PlaygroundState {
  ready: boolean
  backends: BackendInfo[]
  backendId: string
  exampleId: string
  source: string
  mode: ViewMode
  output: RenderResult
  tokens: Token[]
  error: string | null
  init: () => Promise<void>
  setSource: (source: string) => void
  setBackendId: (backendId: string) => void
  setMode: (mode: ViewMode) => void
  loadExample: (exampleId: string) => void
  render: () => void
}

export const usePlaygroundStore = create<PlaygroundState>((set, get) => ({
  ready: false,
  backends: [],
  backendId: 'namuwiki',
  exampleId: EXAMPLES[0].id,
  source: EXAMPLES[0].source,
  mode: 'preview',
  output: { html: '', css: '' },
  tokens: [],
  error: null,

  init: async () => {
    await ensureReady()
    set({ ready: true, backends: listBackends() })
    get().render()
  },

  setSource: (source) => set({ source }),
  setBackendId: (backendId) => set({ backendId }),
  setMode: (mode) => set({ mode }),
  loadExample: (exampleId) => {
    const example = EXAMPLES.find((item) => item.id === exampleId)
    if (example) set({ exampleId, source: example.source })
  },

  render: () => {
    if (!get().ready) return
    try {
      set({
        output: renderMarkup(get().source, get().backendId),
        tokens: inspectTokens(get().source),
        error: null,
      })
    } catch (cause) {
      set({ error: String(cause) })
    }
  },
}))
