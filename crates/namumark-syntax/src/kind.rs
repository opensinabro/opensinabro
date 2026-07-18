/// 무손실 구문 트리의 노드·토큰 종류.
///
/// 토큰 kind는 "원문 조각의 역할"만 나타내고 해석을 담지 않는다.
/// 해석(색상 값, colspan, 앵커 등)은 lowering 단계가 토큰 텍스트에서 계산한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // ---- 토큰 ----
    /// 본문 텍스트 조각 (마커로 소비되지 않은 모든 문자)
    Text = 0,
    /// 개행 (`\n` 또는 `\r\n`)
    Newline,
    /// 백슬래시 + 이스케이프된 문자 1개
    Escaped,
    /// 구문 표식 (헤딩 `==`, 표 `||`·셀 옵션, `{{{#!wiki` 헤더 등 — 아직 세분화되지 않은 것)
    Marker,
    /// 여는 구획 표식 (`'''`·`[[`·`[`·`[*`·`{{{`·`@`)
    DelimiterOpen,
    /// 닫는 구획 표식 (`'''`·`]]`·`]`·`}}}`·`@`)
    DelimiterClose,
    /// 구획 내부 구분자 (링크·이미지의 `|`, 매크로의 `(`·`)`, 변수의 `=`, 각주 이름 뒤 공백 등)
    Separator,
    /// 링크·이미지·분류의 대상 (`[[대상|…]]`의 `대상`)
    LinkTarget,
    /// 매크로 이름 (`[math(…)]`의 `math`)
    MacroName,
    /// 매크로 인자 (`[math(E=mc^2)]`의 `E=mc^2`)
    MacroArgument,
    /// 각주 이름 (`[*이름 …]`의 `이름`)
    FootnoteName,
    /// 틀 인자 이름 (`@이름=기본@`의 `이름`)
    VariableName,
    /// 틀 인자 기본값 (`@이름=기본@`의 `기본`)
    VariableDefault,
    /// 색상 값 (`{{{#ff0000 …}}}`의 `#ff0000`, 다크쌍 `#fff,#000` 포함)
    ColorValue,
    /// 크기 단계 (`{{{+1 …}}}`의 `+1`)
    SizeLevel,
    /// 지시자 (`{{{#!html …}}}`의 `#!html`)
    Directive,
    /// 인용 줄머리의 `>` (+뒤따르는 공백 1개)
    QuoteMarker,
    /// 들여쓰기/리스트 중첩으로 소비된 선행 공백
    IndentMarker,
    /// 리스트 항목 마커 (`* `, `1.#42 ` 등)
    ListMarker,

    // ---- 블록 노드 ----
    Document,
    Paragraph,
    Heading,
    HorizontalRule,
    Quote,
    List,
    ListItem,
    Indent,
    Table,
    TableCaption,
    TableRow,
    TableCell,
    CodeBlock,
    WikiStyle,
    Folding,
    FoldingSummary,
    HtmlBlock,
    ColoredBlock,
    SizedBlock,
    Conditional,
    ConditionExpression,
    Comment,
    Redirect,

    // ---- 인라인 노드 ----
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Superscript,
    Subscript,
    Literal,
    ColoredText,
    SizedText,
    InlineHtml,
    Link,
    Image,
    Category,
    Footnote,
    MacroCall,
    /// `@이름@` / `@이름=기본값@` — 틀 인자
    TemplateVariable,

    // ---- 내부용 ----
    /// 버려진 노드 표시. sink가 건너뛰므로 트리에 나타나지 않는다.
    Tombstone,
}

impl SyntaxKind {
    const LAST: u16 = SyntaxKind::Tombstone as u16;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamumarkLanguage {}

impl rowan::Language for NamumarkLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        assert!(raw.0 <= SyntaxKind::LAST);
        // 안전: repr(u16) 필드 없는 enum이며 범위를 검증했다.
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<NamumarkLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<NamumarkLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<NamumarkLanguage>;
