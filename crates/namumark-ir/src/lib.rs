//! 나무마크 렌더링 IR 타입과 백엔드 계약.
//!
//! resolve pass가 이 타입들을 생성하고(매크로 특화·링크 해석·include 확장 완료),
//! layout pass가 문서 전역 맥락(헤딩 번호, 각주 번호·병합, TOC, 각주 방출 위치)을
//! 확정한다. 백엔드는 layout이 끝난 [`RenderTree`]를 순회만 한다.
//!
//! 표 속성·정렬·리스트 종류는 언어 어휘이므로 의미 모델(namumark-ast)의 타입을 재사용한다.

use namumark_ast::{HorizontalAlignment, ListKind, TableAttribute, VerticalAlignment};

/// 백엔드 계약: layout이 끝난 [`RenderTree`]를 순회해 출력물을 만든다.
pub trait RenderBackend {
    type Output;

    fn render(&self, tree: &RenderTree) -> Self::Output;
}

/// layout까지 끝난 최종 렌더링 입력.
///
/// 모든 노드는 자기완결적이다 — 각주 목록·목차는 해당 블록이 내용을 소유하므로
/// 백엔드는 트리 조회 없이 노드 단위로 방출할 수 있다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTree {
    pub redirect: Option<String>,
    pub blocks: Vec<RenderBlock>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableOfContentsEntry {
    /// "1.2.3" — 앵커는 `s-{number}`
    pub number: String,
    pub depth: u8,
    pub title: Vec<RenderInline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFootnote {
    /// 화면 표기 이름. 무명 각주는 순번("1"), 이름 각주는 그 이름("A").
    pub label: String,
    /// 본문에서 이 각주를 참조한 횟수 (역링크 개수)
    pub reference_count: usize,
    pub content: Vec<RenderInline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderBlock {
    Heading {
        level: u8,
        folded: bool,
        /// layout pass가 채우는 계층 번호 ("1.2"). 앵커는 `s-{number}`.
        number: String,
        content: Vec<RenderInline>,
    },
    Paragraph(Vec<RenderInline>),
    HorizontalRule,
    Quote(Vec<RenderBlock>),
    List {
        kind: ListKind,
        items: Vec<RenderListItem>,
    },
    Indent(Vec<RenderBlock>),
    Table(RenderTable),
    CodeBlock {
        language: Option<String>,
        source: String,
    },
    WikiStyle {
        style: Option<String>,
        dark_style: Option<String>,
        blocks: Vec<RenderBlock>,
    },
    Folding {
        summary: Vec<RenderInline>,
        blocks: Vec<RenderBlock>,
    },
    Colored {
        color: Color,
        blocks: Vec<RenderBlock>,
    },
    Sized {
        level: i8,
        blocks: Vec<RenderBlock>,
    },
    Html(String),
    /// `[목차]` 자리. layout pass가 문서 전체 목차를 채운다.
    TableOfContents {
        entries: Vec<TableOfContentsEntry>,
    },
    /// `[각주]` 자리와 문서 끝 잔여 각주. layout pass가 그 시점까지의 각주를 채운다.
    FootnoteSection {
        notes: Vec<RenderedFootnote>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderListItem {
    pub start_number: Option<u32>,
    pub blocks: Vec<RenderBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTable {
    pub caption: Option<Vec<RenderInline>>,
    pub rows: Vec<RenderTableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTableRow {
    pub cells: Vec<RenderTableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTableCell {
    pub column_span: u32,
    pub row_span: u32,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub attributes: Vec<TableAttribute>,
    pub blocks: Vec<RenderBlock>,
}

/// 라이트/다크 듀얼 색상.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color {
    pub light: ColorValue,
    pub dark: Option<ColorValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorValue {
    /// CSS 색상 이름 ("red")
    Named(String),
    Rgb {
        red: u8,
        green: u8,
        blue: u8,
    },
}

impl ColorValue {
    /// 나무마크 색상 표기("#fff", "#ff0000", "red")를 해석한다.
    /// hex가 아니면 이름 색상으로 그대로 보존한다.
    pub fn parse(source: &str) -> ColorValue {
        let trimmed = source.trim();
        if let Some(hex) = trimmed.strip_prefix('#')
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            let expanded = match hex.len() {
                3 => hex.bytes().flat_map(|byte| [byte, byte]).collect(),
                6 => hex.as_bytes().to_vec(),
                _ => return ColorValue::Named(trimmed.to_string()),
            };
            let component = |index: usize| {
                u8::from_str_radix(
                    std::str::from_utf8(&expanded[index..index + 2]).unwrap(),
                    16,
                )
                .unwrap()
            };
            return ColorValue::Rgb {
                red: component(0),
                green: component(2),
                blue: component(4),
            };
        }
        ColorValue::Named(trimmed.to_string())
    }
}

impl std::fmt::Display for ColorValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorValue::Named(name) => formatter.write_str(name),
            ColorValue::Rgb { red, green, blue } => {
                write!(formatter, "#{red:02x}{green:02x}{blue:02x}")
            }
        }
    }
}

/// 크기 값 ("550" → 픽셀, "100%" → 백분율, 그 외 CSS 표기는 그대로).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dimension {
    Pixels(u32),
    Percentage(u32),
    Custom(String),
}

impl Dimension {
    pub fn parse(source: &str) -> Dimension {
        let trimmed = source.trim();
        if let Some(digits) = trimmed.strip_suffix('%')
            && let Ok(value) = digits.parse()
        {
            return Dimension::Percentage(value);
        }
        if let Some(digits) = trimmed.strip_suffix("px")
            && let Ok(value) = digits.parse()
        {
            return Dimension::Pixels(value);
        }
        if let Ok(value) = trimmed.parse() {
            return Dimension::Pixels(value);
        }
        Dimension::Custom(trimmed.to_string())
    }
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dimension::Pixels(value) => write!(formatter, "{value}px"),
            Dimension::Percentage(value) => write!(formatter, "{value}%"),
            Dimension::Custom(value) => formatter.write_str(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Superscript,
    Subscript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoProvider {
    Youtube,
    KakaoTv,
    NicoVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageLayout {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub align: Option<ImageAlignment>,
    pub background_color: Option<ColorValue>,
    pub theme: Option<ImageTheme>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderInline {
    Text(String),
    LineBreak,
    Styled {
        style: TextStyle,
        content: Vec<RenderInline>,
    },
    Literal(String),
    Colored {
        color: Color,
        content: Vec<RenderInline>,
    },
    Sized {
        level: i8,
        content: Vec<RenderInline>,
    },
    DocumentLink {
        title: String,
        anchor: Option<String>,
        exists: bool,
        display: Option<Vec<RenderInline>>,
    },
    ExternalLink {
        url: String,
        display: Option<Vec<RenderInline>>,
    },
    Image {
        file_name: String,
        url: Option<String>,
        layout: ImageLayout,
    },
    /// resolve 출력 상태의 각주. layout pass가 [`RenderInline::FootnoteReference`]로 치환하므로
    /// 백엔드에는 나타나지 않는다.
    Footnote {
        name: Option<String>,
        content: Vec<RenderInline>,
    },
    FootnoteReference {
        /// 화면 표기 이름 (무명 각주는 순번 "1", 이름 각주는 그 이름)
        label: String,
        /// 같은 각주에 대한 몇 번째 참조인지 (역링크 id용, 0부터)
        reference_index: usize,
    },
    Video {
        provider: VideoProvider,
        identifier: String,
        width: Option<String>,
        height: Option<String>,
    },
    Ruby {
        content: String,
        ruby: String,
    },
    Math {
        formula: String,
    },
    Anchor {
        name: String,
    },
    ClearFix,
    Html(String),
    /// 해석하지 못한 매크로. 화면 일치를 위해 원문 표기로 방출한다.
    Unresolved {
        name: String,
        argument: Option<String>,
    },
}
