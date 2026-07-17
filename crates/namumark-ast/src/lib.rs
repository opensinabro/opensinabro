//! 나무마크 의미 모델 타입.
//!
//! 파서(lowering)가 생성하고 렌더링 pass가 소비하는 문서 구조다.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// 틀 인자(`@이름@`)가 낄 수 있는 문자열.
///
/// 인자는 나무마크 구조를 만들지 않으므로(문법 도움말: 나무마크 자체 문법엔 매개변수를
/// 쓸 수 없다) 값이 정해지기 전에도 구조는 확정된다. 값 결정은 렌더 단계의 몫이다.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Template(pub Vec<Fragment>);

impl Template {
    /// 인자가 끼지 않은 평범한 문자열이면 그 내용.
    pub fn as_literal(&self) -> Option<&str> {
        match self.0.as_slice() {
            [] => Some(""),
            [Fragment::Text(text)] => Some(text),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|fragment| match fragment {
            Fragment::Text(text) => text.is_empty(),
            Fragment::Variable(_) => false,
        })
    }
}

impl From<&str> for Template {
    fn from(text: &str) -> Template {
        if text.is_empty() {
            return Template::default();
        }
        Template(vec![Fragment::Text(text.to_string())])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragment {
    Text(String),
    Variable(Variable),
}

/// `@이름@` 또는 `@이름=기본값@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    /// 인자가 넘어오지 않았을 때 쓸 값. 생략하면 빈 문자열이다.
    pub default: Option<String>,
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
    Comment(String),
    Redirect(Template),
}

/// `{{{#!if 조건식 ...}}}` — 조건이 참일 때만 내용을 렌더한다.
///
/// 조건식은 나무마크가 아니라 틀(include) 인자를 다루는 별도의 작은 표현식 언어다.
/// 대입으로 변수를 만들 수 있고, 내용의 `@이름@`이 그 변수를 참조한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conditional {
    pub expression: String,
    pub blocks: Vec<Block>,
}

/// `{{{#!wiki}}}` — 나무위키에서 이 그룹은 인라인 요소이지만 안에 블록(표 등)을 품는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiStyle {
    pub style: Option<Template>,
    pub dark_style: Option<Template>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folding {
    /// 접기 문구. 위키 문법이 적용되지 않아 글자 그대로다(틀 인자만 값이 된다).
    pub summary: Template,
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
    /// 지정하지 않았으면 None (기본 1, `colspan`·`rowspan` 미방출).
    pub column_span: Option<u32>,
    pub row_span: Option<u32>,
    /// 정렬을 지정하지 않은 셀은 None이다 — 기본(왼쪽)이며 나무위키는 이때
    /// `text-align`을 방출하지 않는다.
    pub horizontal_alignment: Option<HorizontalAlignment>,
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
    pub value: Option<Template>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAttributeScope {
    Cell,
    /// 열 지정. `columns`는 이 옵션을 적은 자리까지 아는 칸 수다 — 나무위키는 셀 옵션을
    /// 왼쪽부터 처리해서, `<-3><colbgcolor=…>`는 세 열에 걸리지만
    /// `<colbgcolor=…><-4>`는 적힌 시점에 한 칸뿐이라 한 열에만 걸린다(렌더확정).
    Column {
        columns: u32,
    },
    Row,
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
    /// 텍스트 문맥의 `@이름@`.
    Variable(Variable),
    Colored(ColoredText),
    Sized(SizedText),
    /// `{{{#!wiki style="…" … }}}`
    WikiStyle(WikiStyle),
    /// `{{{#!folding 문구 … }}}`
    Folding(Folding),
    /// `{{{#!if 조건식 … }}}`
    Conditional(Conditional),
    /// `{{{#!syntax 언어 … }}}` 또는 지시자 없는 여러 줄 `{{{ … }}}`
    CodeBlock(CodeBlock),
    /// `{{{#!html … }}}`
    Html(Template),
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
    pub target: Template,
    /// `[[문서#s-1]]`의 `s-1`. 외부 URL은 분리하지 않는다.
    pub anchor: Option<Template>,
    pub display: Option<Vec<Inline>>,
}

/// `[[파일:이름|width=100%&align=center]]` 이미지 삽입.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub file_name: Template,
    pub options: Vec<ImageOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOption {
    pub name: String,
    pub value: Option<Template>,
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
    pub argument: Option<Template>,
}

/// 원문 표기를 되살린다. 값이 정해지기 전이므로 인자는 `@이름=기본값@` 그대로다.
impl std::fmt::Display for Template {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for fragment in &self.0 {
            match fragment {
                Fragment::Text(text) => formatter.write_str(text)?,
                Fragment::Variable(variable) => write!(formatter, "{variable}")?,
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Variable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.default {
            Some(default) => write!(formatter, "@{}={default}@", self.name),
            None => write!(formatter, "@{}@", self.name),
        }
    }
}
