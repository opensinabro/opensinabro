//! 나무마크 렌더링 IR 타입과 백엔드 계약.
//!
//! resolve pass가 이 타입들을 생성하고(매크로 특화·링크 해석·include 확장 완료),
//! layout pass가 문서 전역 맥락(헤딩 번호, 각주 번호·병합, TOC, 각주 방출 위치)을
//! 확정한다. 백엔드는 layout이 끝난 [`RenderTree`]를 순회만 한다.
//!
//! 표 속성·정렬·리스트 종류는 언어 어휘이므로 의미 모델(namumark-ast)의 타입을 재사용한다.

use namumark_ast::{HorizontalAlignment, ListKind, TableAttributeScope, VerticalAlignment};

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
    /// 헤딩 제목 그대로. 링크도 살아 있다(렌더확정: `== [[/TeX|수식]] ==`의 목차 항목이
    /// the seed에서도 `<a href='/w/…/TeX'>수식</a>`이다).
    pub title: Vec<RenderInline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFootnote {
    /// 화면 표기 이름. 무명 각주는 참조 번호("16"), 이름 각주는 그 이름("A").
    pub label: String,
    /// 본문에서 이 각주를 참조한 자리들의 번호. 역링크가 여기로 되돌아간다.
    pub reference_numbers: Vec<u32>,
    pub content: Vec<RenderInline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderBlock {
    Heading {
        level: u8,
        folded: bool,
        /// layout pass가 채우는 계층 번호 ("1.2"). 앵커는 `s-{number}`.
        number: String,
        /// 문단명 앵커. `[[#개요]]`로 걸 수 있도록 제목 글자를 그대로 쓴다.
        anchor: String,
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

/// 표 속성. AST와 달리 틀 인자가 이미 값으로 확정돼 있다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTableAttribute {
    pub scope: TableAttributeScope,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTableRow {
    pub cells: Vec<RenderTableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTableCell {
    /// 지정하지 않았으면 None (기본 1, `colspan`·`rowspan` 미방출).
    pub column_span: Option<u32>,
    pub row_span: Option<u32>,
    /// 지정하지 않았으면 None (기본 왼쪽, `text-align` 미방출).
    pub horizontal_alignment: Option<HorizontalAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub attributes: Vec<RenderTableAttribute>,
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
    ///
    /// 색이 아니면 None이다 — 나무위키는 아무 문자열이나 색으로 받지 않고, 색이 아닌
    /// 값이 들어간 선언은 통째로 버린다(`<bgcolor=#배경색>`처럼 틀 인자가 안 채워진 경우).
    pub fn parse(source: &str) -> Option<ColorValue> {
        let trimmed = source.trim();
        if let Some(hex) = trimmed.strip_prefix('#')
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            let expanded: Vec<u8> = match hex.len() {
                3 => hex.bytes().flat_map(|byte| [byte, byte]).collect(),
                6 => hex.as_bytes().to_vec(),
                _ => return None,
            };
            let component = |index: usize| {
                u8::from_str_radix(
                    std::str::from_utf8(&expanded[index..index + 2]).unwrap(),
                    16,
                )
                .unwrap()
            };
            return Some(ColorValue::Rgb {
                red: component(0),
                green: component(2),
                blue: component(4),
            });
        }
        // `transparent`는 색상 이름이 아니라 CSS 색상 키워드지만 색 자리에 올 수 있다
        // (렌더확정: `<tablebgcolor=transparent>` → `background-color:transparent`).
        (namumark_text::is_css_color_name(trimmed) || trimmed.eq_ignore_ascii_case("transparent"))
            .then(|| ColorValue::Named(trimmed.to_string()))
    }

    /// 이미 색으로 판정된 표기를 해석한다. 문법이 `{{{#색상 …}}}`을 색상 그룹으로
    /// 인정했다는 것은 판정을 거쳤다는 뜻이라, 여기서 다시 물리지 않는다.
    pub fn parse_known(source: &str) -> ColorValue {
        ColorValue::parse(source).unwrap_or_else(|| ColorValue::Named(source.trim().to_string()))
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

/// 숫자를 픽셀 정수로 읽는다. 소수는 버리고, 음수·비숫자는 픽셀이 아니다.
fn whole_pixels(digits: &str) -> Option<u32> {
    let value: f64 = digits.trim().parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value.trunc() as u32)
}

impl Dimension {
    pub fn parse(source: &str) -> Dimension {
        let trimmed = source.trim();
        if let Some(digits) = trimmed.strip_suffix('%')
            && let Ok(value) = digits.parse()
        {
            return Dimension::Percentage(value);
        }
        // 나무위키는 픽셀 값의 소수점을 버린다(`width=21.6px` → `21px`).
        if let Some(digits) = trimmed.strip_suffix("px")
            && let Some(value) = whole_pixels(digits)
        {
            return Dimension::Pixels(value);
        }
        // 단위 없는 값은 정수일 때만 픽셀이다. 소수는 그대로 나간다
        // (렌더확정: `<width=33.3>` → `width:33.3`, `<width=1000>` → `width:1000px`).
        if !trimmed.is_empty()
            && trimmed.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(value) = trimmed.parse()
        {
            return Dimension::Pixels(value);
        }
        Dimension::Custom(trimmed.to_string())
    }

    /// 이미지 크기 전용 파싱. 표 셀과 달리 단위 없는 소수도 픽셀로 잘라 낸다
    /// (렌더확정: 이미지 `width=21.6` → `width:21px`, 표 셀 `<width=33.3>` → `width:33.3`).
    pub fn parse_image(source: &str) -> Dimension {
        let trimmed = source.trim();
        if let Some(digits) = trimmed.strip_suffix('%')
            && let Ok(value) = digits.parse()
        {
            return Dimension::Percentage(value);
        }
        if let Some(value) = whole_pixels(trimmed.strip_suffix("px").unwrap_or(trimmed)) {
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

/// 문서 링크가 가리키는 곳의 성격. 나무위키는 셋을 다르게 꾸민다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLinkKind {
    Existing,
    /// 아직 없는 문서. 검색 엔진이 따라가지 않게 한다.
    Missing,
    /// 지금 보고 있는 문서 자신.
    Current,
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
        kind: DocumentLinkKind,
        /// 화면에 나오는 글자. 표시부를 안 적었으면 resolve가 적힌 대상으로 채운다.
        display: Vec<RenderInline>,
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
        /// 화면 표기 이름 (무명 각주는 참조 번호 "16", 이름 각주는 그 이름)
        label: String,
        /// 문서 안 모든 각주 참조에 차례로 붙는 번호. 재참조도 제 번호를 받는다.
        /// 무명 각주의 라벨이 곧 이 번호다(렌더확정: `[A]`가 13·14를 쓰면 다음
        /// 무명 각주는 13이 아니라 16이다).
        number: u32,
        /// 각주 내용의 글자만 뽑은 것. 나무위키는 이걸 `title`에 실어 툴팁으로 보여준다.
        tooltip: String,
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
        /// `color=`로 준 루비 글자색. 나무위키는 `<rt>` 안을 span으로 감싼다.
        color: Option<ColorValue>,
    },
    Math {
        formula: String,
    },
    Anchor {
        name: String,
    },
    ClearFix,
    /// `[목차]` 자리. layout pass가 문서 전체 목차를 채운다.
    TableOfContents {
        entries: Vec<TableOfContentsEntry>,
    },
    /// `[각주]` 자리와 문서 끝 잔여 각주. layout pass가 그 시점까지의 각주를 채운다.
    FootnoteSection {
        notes: Vec<RenderedFootnote>,
    },
    /// 감싸는 요소 없이 문단 안에 놓이는 블록들. `#!if`가 표·리스트를 품을 때 쓴다 —
    /// 조건은 내용을 가릴 뿐 자기 요소를 만들지 않는다.
    Blocks(Vec<RenderBlock>),
    /// `{{{#!wiki}}}` — 나무위키에서 인라인이지만 안에 블록을 품는다.
    WikiStyle {
        style: Option<String>,
        dark_style: Option<String>,
        blocks: Vec<RenderBlock>,
    },
    Folding {
        /// 접기 문구. 위키 문법이 적용되지 않아 글자 그대로다.
        summary: String,
        blocks: Vec<RenderBlock>,
    },
    CodeBlock {
        language: Option<String>,
        source: String,
    },
    Html(String),
    /// 해석하지 못한 매크로. 화면 일치를 위해 원문 표기로 방출한다.
    Unresolved {
        name: String,
        argument: Option<String>,
    },
}
