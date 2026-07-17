import { create } from 'zustand'
import {
  ensureReady,
  listBackends,
  renderMarkup,
  type BackendInfo,
  type RenderResult,
} from '@/lib/wasm'
import { EXAMPLES } from '@/examples'

interface PlaygroundState {
  ready: boolean
  backends: BackendInfo[]
  backendId: string
  exampleId: string
  source: string
  output: RenderResult
  error: string | null
  init: () => Promise<void>
  setSource: (source: string) => void
  setBackendId: (backendId: string) => void
  loadExample: (exampleId: string) => void
  render: () => void
}

export const usePlaygroundStore = create<PlaygroundState>((set, get) => ({
  ready: false,
  backends: [],
  backendId: 'namuwiki',
  exampleId: EXAMPLES[0].id,
  source: EXAMPLES[0].source,
  output: { html: '', css: '' },
  error: null,

  init: async () => {
    await ensureReady()
    set({ ready: true, backends: listBackends() })
    get().render()
  },

  setSource: (source) => set({ source }),
  setBackendId: (backendId) => set({ backendId }),
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
