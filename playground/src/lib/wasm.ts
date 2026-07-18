import init, {
  render as wasmRender,
  backends,
  inspect as wasmInspect,
  diagnose as wasmDiagnose,
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

export type DiagnosticSeverity = 'warning' | 'suggestion' | 'info'

export interface Diagnostic {
  /** 안정적 식별자: redirect-trailing-content·unsupported-macro … */
  code: string
  severity: DiagnosticSeverity
  /** correctness·deprecation·style·unsupported */
  category: string
  message: string
  /** 원문 바이트 오프셋 [start, end). */
  start: number
  end: number
}

/**
 * 원문을 검사해 진단을 원문 위치 순으로 돌려준다. 문맥 자유 검사(리다이렉트 후행
 * 내용 등)와 resolve의 문맥 의존 검사(미지원 매크로)를 합친 결과다.
 */
export function diagnose(source: string): Diagnostic[] {
  return JSON.parse(wasmDiagnose(source)) as Diagnostic[]
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
