import type { Token } from '@/lib/wasm'

/** 토큰 역할(SyntaxKind) → 한국어. */
const KIND_LABEL: Record<string, string> = {
  Text: '텍스트',
  Newline: '개행',
  Escaped: '이스케이프',
  Marker: '표식',
  DelimiterOpen: '여는 표식',
  DelimiterClose: '닫는 표식',
  Separator: '구분자',
  LinkTarget: '대상',
  MacroName: '매크로 이름',
  MacroArgument: '매크로 인자',
  FootnoteName: '각주 이름',
  VariableName: '변수 이름',
  VariableDefault: '변수 기본값',
  ColorValue: '색상 값',
  SizeLevel: '크기 단계',
  Directive: '지시자',
  QuoteMarker: '인용 표식',
  IndentMarker: '들여쓰기',
  ListMarker: '리스트 표식',
}

/** 부모 노드(SyntaxKind) → 문맥 한국어. */
const PARENT_LABEL: Record<string, string> = {
  Document: '문서',
  Paragraph: '문단',
  Heading: '문단 제목',
  HorizontalRule: '가로줄',
  Quote: '인용문',
  List: '리스트',
  ListItem: '리스트 항목',
  Indent: '들여쓰기',
  Table: '표',
  TableCaption: '표 캡션',
  TableRow: '표 행',
  TableCell: '표 셀',
  CodeBlock: '코드 블록',
  WikiStyle: 'wiki 상자',
  Folding: '접기',
  FoldingSummary: '접기 요약',
  HtmlBlock: 'HTML 블록',
  ColoredBlock: '색 상자',
  SizedBlock: '크기 상자',
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
  Literal: '리터럴',
  ColoredText: '색 지정',
  SizedText: '크기 지정',
  InlineHtml: '인라인 HTML',
  Link: '링크',
  Image: '이미지',
  Category: '분류',
  Footnote: '각주',
  MacroCall: '매크로',
  TemplateVariable: '틀 변수',
  Tombstone: '폐기',
}

/** 색상 그룹. 같은 계열 노드는 같은 색으로 묶는다. */
const CATEGORY: Record<string, string> = {
  Bold: 'format',
  Italic: 'format',
  Strikethrough: 'format',
  Underline: 'format',
  Superscript: 'format',
  Subscript: 'format',
  ColoredText: 'style',
  SizedText: 'style',
  ColoredBlock: 'style',
  SizedBlock: 'style',
  Link: 'link',
  Image: 'link',
  Category: 'link',
  Heading: 'heading',
  Table: 'table',
  TableCaption: 'table',
  TableRow: 'table',
  TableCell: 'table',
  CodeBlock: 'code',
  WikiStyle: 'code',
  Literal: 'code',
  HtmlBlock: 'code',
  InlineHtml: 'code',
  MacroCall: 'macro',
  Footnote: 'macro',
  TemplateVariable: 'macro',
  Conditional: 'macro',
  ConditionExpression: 'macro',
  Quote: 'flow',
  List: 'flow',
  ListItem: 'flow',
  Indent: 'flow',
  Folding: 'flow',
  FoldingSummary: 'flow',
}

export const CATEGORY_COLOR: Record<string, string> = {
  format: '#7c3aed',
  style: '#db2777',
  link: '#2563eb',
  heading: '#0d9488',
  table: '#ea580c',
  code: '#78716c',
  macro: '#4f46e5',
  flow: '#0891b2',
  plain: '#64748b',
}

export interface TokenMeaning {
  /** "굵게 · 표식" 같은 사람이 읽는 의미. */
  label: string
  /** 색상 그룹 키 (CATEGORY_COLOR의 키). */
  category: string
  color: string
}

export function describe(token: Token): TokenMeaning {
  const kindLabel = KIND_LABEL[token.kind] ?? token.kind
  const parentLabel = PARENT_LABEL[token.parent] ?? token.parent
  const category = CATEGORY[token.parent] ?? 'plain'
  const label =
    token.parent === 'Paragraph' && token.kind === 'Text'
      ? '본문 텍스트'
      : `${parentLabel} · ${kindLabel}`
  return { label, category, color: CATEGORY_COLOR[category] }
}
