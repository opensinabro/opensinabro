import init, { render as wasmRender, backends } from '../../wasm/namumark_playground.js'

export interface BackendInfo {
  id: string
  label: string
}

export interface RenderResult {
  html: string
  css: string
}

let readyPromise: Promise<unknown> | undefined

/** WASM 모듈을 한 번만 초기화한다. */
export function ensureReady(): Promise<unknown> {
  if (!readyPromise) {
    readyPromise = init()
  }
  return readyPromise
}

/** 지원 백엔드 목록. `ensureReady()` 이후에만 호출한다. */
export function listBackends(): BackendInfo[] {
  return JSON.parse(backends()) as BackendInfo[]
}

/** 나무마크 원문을 렌더해 `{ html, css }`를 돌려준다. */
export function renderMarkup(source: string, backendId: string): RenderResult {
  const output = wasmRender(source, backendId)
  try {
    return { html: output.html, css: output.css }
  } finally {
    output.free()
  }
}
