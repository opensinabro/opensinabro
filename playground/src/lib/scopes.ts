import type { Token } from '@/lib/wasm'

/** 구문 하이라이트가 색을 입히는 의미 범주. 테마는 이 스코프만 칠한다. */
export const SCOPES = [
  'text',
  'punctuation',
  'heading',
  'strong',
  'emphasis',
  'underline',
  'strike',
  'link',
  'macro',
  'code',
  'table',
  'attribute',
  'comment',
] as const

export type Scope = (typeof SCOPES)[number]

/** 구조 표식·구분자. 내용과 분리해 흐리게 죽인다. */
const PUNCTUATION_KINDS = new Set([
  'Marker',
  'DelimiterOpen',
  'DelimiterClose',
  'Separator',
  'QuoteMarker',
  'IndentMarker',
  'ListMarker',
  'ListStartNumber',
  'AlignmentSpace',
  'Directive',
])

/** 속성·값 계열 리프. */
const ATTRIBUTE_KINDS = new Set([
  'AttributeName',
  'AttributeValue',
  'CellOption',
  'CellOptionName',
  'CellOptionValue',
  'ColorValue',
  'SizeLevel',
  'ArgumentName',
  'ArgumentValue',
  'CodeLanguage',
])

const LINK_KINDS = new Set(['LinkTarget', 'LinkNamespace', 'LinkAnchor'])
const MACRO_KINDS = new Set(['MacroName', 'MacroArgument', 'FootnoteName', 'VariableName', 'VariableDefault'])

/** 부모 노드로 결정되는 스코프. */
const PARENT_SCOPE: Record<string, Scope> = {
  Heading: 'heading',
  Bold: 'strong',
  Italic: 'emphasis',
  Underline: 'underline',
  Strikethrough: 'strike',
  Link: 'link',
  Image: 'link',
  Category: 'link',
  MacroCall: 'macro',
  Footnote: 'macro',
  TemplateVariable: 'macro',
  Conditional: 'macro',
  ConditionExpression: 'macro',
  CodeBlock: 'code',
  WikiStyle: 'code',
  Literal: 'code',
  HtmlBlock: 'code',
  InlineHtml: 'code',
  Table: 'table',
  TableCaption: 'table',
  TableRow: 'table',
  TableCell: 'table',
  Comment: 'comment',
}

/** 토큰 하나를 의미 스코프로 접는다. kind(역할)를 먼저 보고, 없으면 parent(문맥)로. */
export function scopeOf(token: Token): Scope {
  if (PUNCTUATION_KINDS.has(token.kind)) return 'punctuation'
  if (ATTRIBUTE_KINDS.has(token.kind)) return 'attribute'
  if (LINK_KINDS.has(token.kind)) return 'link'
  if (MACRO_KINDS.has(token.kind)) return 'macro'
  if (token.parent === 'Comment') return 'comment'
  return PARENT_SCOPE[token.parent] ?? 'text'
}

export function scopeClass(token: Token): string {
  return `nm-tok-${scopeOf(token)}`
}

/** 노드(부모) → 사람이 읽는 의미. 코드상의 노드 이름 대신 이걸 툴팁에 보여준다. */
const NODE_LABEL: Record<string, string> = {
  Heading: '제목',
  Paragraph: '본문',
  Quote: '인용문',
  List: '리스트',
  ListItem: '리스트 항목',
  Indent: '들여쓰기',
  HorizontalRule: '가로줄',
  Table: '표',
  TableCaption: '표 캡션',
  TableRow: '표 행',
  TableCell: '표 셀',
  CodeBlock: '코드 블록',
  WikiStyle: 'wiki 상자',
  Folding: '접기',
  FoldingSummary: '접기 요약',
  HtmlBlock: 'HTML 블록',
  InlineHtml: '인라인 HTML',
  Literal: '리터럴',
  ColoredBlock: '색 상자',
  SizedBlock: '크기 상자',
  ColoredText: '색 지정',
  SizedText: '크기 지정',
  Conditional: '조건식',
  ConditionExpression: '조건',
  Comment: '주석',
  Redirect: '넘겨주기',
  Bold: '굵게',
  Italic: '기울임',
  Strikethrough: '취소선',
  Underline: '밑줄',
  Superscript: '윗첨자',
  Subscript: '아래첨자',
  Link: '링크',
  Image: '이미지',
  Category: '분류',
  Footnote: '각주',
  MacroCall: '매크로',
  TemplateVariable: '틀 변수',
}

/** 토큰이 속한 노드의 의미 라벨. */
export function nodeLabel(token: Token): string {
  return NODE_LABEL[token.parent] ?? '텍스트'
}
