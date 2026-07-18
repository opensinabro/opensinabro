import { SCOPES, type Scope } from '@/lib/scopes'

export interface TokenStyle {
  color?: string
  background?: string
  fontWeight?: number
  fontStyle?: 'italic'
  textDecoration?: string
}

/**
 * 구문 하이라이트 스타일시트. 스코프(의미 범주)만 칠하므로, 새 테마는 여기에
 * 항목을 더하기만 하면 UI 드롭다운에 자동 노출된다 — 파서·컴포넌트 무변경.
 * 다크모드는 두지 않으므로 모든 테마는 밝은 바탕을 전제한다.
 *
 * 테마는 '색'만 정한다. 노드 맥락에 맞는 서식(굵게→볼드, 기울임→이탤릭,
 * 링크→밑줄, 코드→배경 등)은 [`BASE_STYLES`]가 테마와 무관하게 공통으로 얹는다.
 */
export interface HighlightTheme {
  id: string
  label: string
  styles: Partial<Record<Scope, TokenStyle>>
}

/**
 * 노드 맥락 서식 — 모든 테마 공통. 원문 글자가 그 서식대로 보이게 해
 * 어떤 노드인지 색 없이도 읽히게 한다. 정렬을 깨는 속성(글자 폭·줄 높이를
 * 바꾸는 것)은 쓰지 않는다 — 배경·밑줄·취소선·(모노 등폭) 굵기·이탤릭만.
 */
const BASE_STYLES: Partial<Record<Scope, TokenStyle>> = {
  heading: { fontWeight: 600 },
  strong: { fontWeight: 700 },
  emphasis: { fontStyle: 'italic' },
  underline: { textDecoration: 'underline' },
  strike: { textDecoration: 'line-through' },
  link: { textDecoration: 'underline' },
  comment: { fontStyle: 'italic' },
  code: { background: 'rgba(20, 24, 30, 0.04)' },
}

/** hover 중인 노드 전체를 덮는 강조색. */
const HOVER_BACKGROUND = 'rgba(47, 107, 71, 0.13)'

export const THEMES: HighlightTheme[] = [
  {
    id: 'neutral',
    label: '기본',
    styles: {
      punctuation: { color: '#adaba3' },
      heading: { color: '#2f6b47' },
      link: { color: '#3a5f8a' },
      macro: { color: '#6b4fa8' },
      code: { color: '#9a6a33' },
      table: { color: '#b0663a' },
      attribute: { color: '#9a6a33' },
      comment: { color: '#a3a59c' },
    },
  },
  {
    id: 'muted',
    label: '저채도',
    styles: {
      punctuation: { color: '#c6c6c0' },
      heading: { color: '#4b5d51' },
      link: { color: '#556472' },
      macro: { color: '#63607a' },
      code: { color: '#7a7060' },
      table: { color: '#7a7060' },
      attribute: { color: '#7a7060' },
      comment: { color: '#b2b4ac' },
    },
  },
  {
    id: 'contrast',
    label: '고대비',
    styles: {
      punctuation: { color: '#8f8f88' },
      heading: { color: '#137a41', fontWeight: 700 },
      link: { color: '#1f5fd0' },
      macro: { color: '#7b2fd0' },
      code: { color: '#b5451b' },
      table: { color: '#c85a1f' },
      attribute: { color: '#b5451b' },
      comment: { color: '#8f9298' },
    },
  },
  {
    id: 'print',
    label: '인쇄',
    styles: {
      punctuation: { color: '#bcbcb6' },
      heading: { color: '#17181a', fontWeight: 700 },
      link: { color: '#17181a' },
      macro: { color: '#3e4046', fontStyle: 'italic' },
      code: { color: '#3e4046' },
      table: { color: '#3e4046' },
      attribute: { color: '#6a6c72' },
      comment: { color: '#a6a8ae' },
    },
  },
]

function declarations(style: TokenStyle): string {
  const parts: string[] = []
  if (style.color) parts.push(`color:${style.color}`)
  if (style.background) parts.push(`background:${style.background}`)
  if (style.fontWeight) parts.push(`font-weight:${style.fontWeight}`)
  if (style.fontStyle) parts.push(`font-style:${style.fontStyle}`)
  if (style.textDecoration) parts.push(`text-decoration:${style.textDecoration}`)
  return parts.join(';')
}

/** 공통 맥락 서식 + 테마별 색 + hover 강조를 한 CSS 문자열로. */
export function themeCss(): string {
  const rules: string[] = []

  for (const scope of SCOPES) {
    const style = BASE_STYLES[scope]
    if (!style) continue
    const declared = declarations(style)
    if (declared) rules.push(`.nm-hl .nm-tok-${scope}{${declared}}`)
  }

  for (const theme of THEMES) {
    for (const scope of SCOPES) {
      const style = theme.styles[scope]
      if (!style) continue
      const declared = declarations(style)
      if (declared) rules.push(`.nm-hl[data-hltheme="${theme.id}"] .nm-tok-${scope}{${declared}}`)
    }
  }

  // 배경이 걸리는 규칙(코드 등)보다 뒤에 둬 hover 강조가 이긴다.
  rules.push(`.nm-hl .nm-tok-hover{background:${HOVER_BACKGROUND}}`)

  return rules.join('\n')
}
