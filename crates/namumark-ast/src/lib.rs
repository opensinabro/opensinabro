//! 나무마크 의미 모델 타입.
//!
//! 파서(lowering)가 생성하고 렌더링 pass가 소비하는 문서 구조다.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(Heading),
    Paragraph(Vec<Inline>),
    HorizontalRule,
    Quote(Vec<Block>),
    List(List),
    Indent(Vec<Block>),
    Table(Table),
    CodeBlock(CodeBlock),
    WikiStyle(WikiStyle),
    Folding(Folding),
    Colored(ColoredBlock),
    Sized(SizedBlock),
    Html(String),
    Comment(String),
    Redirect(String),
}

/// `{{{#색상 ...}}}`이 여러 줄에 걸쳐 블록(표, 리스트 등)을 감싸는 형태.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColoredBlock {
    pub color: String,
    pub dark_color: Option<String>,
    pub blocks: Vec<Block>,
}

/// `{{{+1 ...}}}`이 여러 줄에 걸쳐 블록을 감싸는 형태.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizedBlock {
    pub level: i8,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiStyle {
    pub style: Option<String>,
    pub dark_style: Option<String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folding {
    pub summary: Vec<Inline>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub caption: Option<Vec<Inline>>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCell {
    pub column_span: u32,
    pub row_span: u32,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub attributes: Vec<TableAttribute>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableAttribute {
    pub scope: TableAttributeScope,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAttributeScope {
    Cell,
    Row,
    Column,
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub folded: bool,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    pub kind: ListKind,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Unordered,
    Decimal,
    LowerAlphabet,
    UpperAlphabet,
    LowerRoman,
    UpperRoman,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    /// `1.#42`처럼 순서 리스트의 번호를 재지정하는 경우에만 값이 있다.
    pub start_number: Option<u32>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    LineBreak,
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Underline(Vec<Inline>),
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    Literal(String),
    Link(Link),
    Image(Image),
    Category(Category),
    Footnote(Footnote),
    Macro(Macro),
    Colored(ColoredText),
    Sized(SizedText),
    Html(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColoredText {
    pub color: String,
    pub dark_color: Option<String>,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizedText {
    pub level: i8,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub target: String,
    /// `[[문서#s-1]]`의 `s-1`. 외부 URL은 분리하지 않는다.
    pub anchor: Option<String>,
    pub display: Option<Vec<Inline>>,
}

/// `[[파일:이름|width=100%&align=center]]` 이미지 삽입.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub file_name: String,
    pub options: Vec<ImageOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOption {
    pub name: String,
    pub value: Option<String>,
}

/// `[[분류:이름]]` 분류 등록. 본문에는 표시되지 않는 메타데이터다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footnote {
    pub name: Option<String>,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro {
    pub name: String,
    pub argument: Option<String>,
}
