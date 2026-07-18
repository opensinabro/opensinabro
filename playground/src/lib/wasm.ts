import init, {
  render as wasmRender,
  backends,
  inspect as wasmInspect,
} from '../../wasm/namumark_playground.js'

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

export interface Token {
  /** 토큰의 역할 (SyntaxKind): Marker·Text·ListMarker … */
  kind: string
  /** 토큰을 감싼 노드 (SyntaxKind): Bold·Heading·Link … */
  parent: string
  /** 원문에서의 바이트 시작 위치 */
  start: number
  /** 토큰 원문 조각. 순서대로 이으면 원문이 복원된다. */
  text: string
}

/** 무손실 구문 트리의 리프 토큰을 원문 순서대로 돌려준다. */
export function inspectTokens(source: string): Token[] {
  return JSON.parse(wasmInspect(source)) as Token[]
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
