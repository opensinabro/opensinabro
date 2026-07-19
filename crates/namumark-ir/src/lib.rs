//! 나무마크 렌더링 IR 타입과 백엔드 계약.
//!
//! resolve pass가 이 타입들을 생성하고(매크로 특화·링크 해석·include 확장 완료),
//! layout pass가 문서 전역 맥락(헤딩 번호, 각주 번호·병합, TOC, 각주 방출 위치)을
//! 확정한다. 백엔드는 layout이 끝난 [`RenderTree`]를 순회만 한다.
//!
//! 표 속성·정렬·리스트 종류는 언어 어휘이므로 의미 모델(namumark-ast)의 타입을 재사용한다.

use namumark_ast::{HorizontalAlignment, ListKind, TableAttributeScope, VerticalAlignment};
use serde::Serialize;
use ts_rs::TS;

mod html;

pub use html::{HtmlAttributes, HtmlNode, HtmlTag};

/// 백엔드 계약: layout이 끝난 [`RenderTree`]를 순회해 출력물을 만든다.
pub trait RenderBackend {
    type Output;

    fn render(&self, tree: &RenderTree) -> Self::Output;
}

/// layout까지 끝난 최종 렌더링 입력.
///
/// 문서 전역 정보(목차·각주)는 트리 밖 최상위 목록이 소유하고 트리 안에는 자리표시만
/// 남는다. 그래야 layout이 헤딩을 다 본 뒤 목차를 채우려고 트리를 다시 훑지 않아도 되고,
/// 여러 번 참조된 각주의 내용이 참조 수만큼 실려 나가지도 않는다. 대신 `[목차]`·`[각주]`를
/// 그리는 쪽은 노드만으로는 부족하고 이 목록을 함께 봐야 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderTree {
    pub redirect: Option<String>,
    pub blocks: Vec<RenderBlock>,
    pub categories: Vec<String>,
    /// 문서 전체 목차. `[목차]` 자리마다 이 목록을 그대로 그린다.
    pub table_of_contents: Vec<TableOfContentsEntry>,
    /// 문서의 모든 각주를 방출 순서대로 담는다. 각주 내용의 유일한 소유자이며,
    /// [`RenderInline::FootnoteSection`]은 이 목록의 인덱스만 가리킨다.
    pub footnotes: Vec<RenderedFootnote>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TableOfContentsEntry {
    /// "1.2.3" — 앵커는 `s-{number}`
    pub number: String,
    pub depth: u8,
    /// 헤딩 제목 그대로. 링크도 살아 있다(렌더확정: `== [[/TeX|수식]] ==`의 목차 항목이
    /// the seed에서도 `<a href='/w/…/TeX'>수식</a>`이다).
    pub title: Vec<RenderInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderedFootnote {
    /// 화면 표기 이름. 무명 각주는 참조 번호("16"), 이름 각주는 그 이름("A").
    pub label: String,
    /// 본문에서 이 각주를 참조한 자리들의 번호. 역링크가 여기로 되돌아간다.
    pub reference_numbers: Vec<u32>,
    pub content: Vec<RenderInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
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
    Paragraph {
        content: Vec<RenderInline>,
    },
    HorizontalRule,
    Quote {
        blocks: Vec<RenderBlock>,
    },
    List {
        kind: ListKind,
        items: Vec<RenderListItem>,
    },
    Indent {
        blocks: Vec<RenderBlock>,
    },
    Table {
        table: RenderTable,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderListItem {
    pub start_number: Option<u32>,
    pub blocks: Vec<RenderBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderTable {
    pub caption: Option<Vec<RenderInline>>,
    pub rows: Vec<RenderTableRow>,
}

/// 표 스타일 속성. 색·크기 표기 해석과 방출 여부 판정을 resolve가 끝냈으므로,
/// 백엔드는 문자열 이름 대조나 값 파싱 없이 확정된 값을 방출만 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderTableAttribute {
    pub scope: TableAttributeScope,
    pub property: TableStyleProperty,
}

/// 값이 이미 파싱된 표 스타일 속성.
///
/// 색은 듀얼 표기(`#fff,#000`)의 라이트 값만 담는다 — 표 색의 다크 모드는 후속 과제다.
/// 색 표기가 아닌 값이 들어온 선언은 resolve가 통째로 버리므로 여기 나타나지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum TableStyleProperty {
    BackgroundColor {
        #[ts(type = "string")]
        color: ColorValue,
    },
    Color {
        #[ts(type = "string")]
        color: ColorValue,
    },
    BorderColor {
        #[ts(type = "string")]
        color: ColorValue,
    },
    Width {
        #[ts(type = "string")]
        width: Dimension,
    },
    Height {
        #[ts(type = "string")]
        height: Dimension,
    },
    /// `text-align`. 나무위키는 left·center·right만 받고 그 외 값은 선언을 통째로 버린다.
    TextAlign { alignment: HorizontalAlignment },
    /// 표 정렬(`<tablealign=…>`). 감싸는 div의 클래스가 된다 — center·right만 클래스를
    /// 만들고, left(기본)·인식 못한 값은 클래스가 없다.
    Align { alignment: HorizontalAlignment },
    /// `<nopad>` — 셀 패딩 제거.
    NoPadding,
}

impl TableStyleProperty {
    /// 표(`table`) 요소의 `style`로 나가는 속성인가.
    pub fn emits_table_style(&self) -> bool {
        !matches!(
            self,
            TableStyleProperty::Align { .. } | TableStyleProperty::NoPadding
        )
    }

    /// 행·열·셀의 `style`로 나가는 속성인가.
    pub fn emits_cell_style(&self) -> bool {
        matches!(
            self,
            TableStyleProperty::BackgroundColor { .. }
                | TableStyleProperty::Color { .. }
                | TableStyleProperty::Width { .. }
                | TableStyleProperty::Height { .. }
                | TableStyleProperty::TextAlign { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderTableRow {
    pub cells: Vec<RenderTableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Color {
    #[ts(type = "string")]
    pub light: ColorValue,
    #[ts(type = "string | null")]
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

/// 값 타입은 이미 정본 CSS 표기를 내는 `Display`가 있고, 어떤 백엔드도 성분을 따로
/// 쓰지 않는다. 그래서 와이어에는 그 표기 그대로 문자열로 나간다 — 받는 쪽이
/// 성분을 다시 조립할 일이 없다.
impl Serialize for ColorValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl Serialize for Dimension {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
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

/// `#!wiki`가 실어 온 CSS를 나무위키가 받아들이는 만큼만 걸러 낸 뒤 남은 선언 하나.
///
/// 위키 입력이 CSS로 나가는 통로라 색·크기와 같은 값 해석 부류다 — 나무위키는 여기를
/// 그냥 흘려보내지 않고 무효·미지원 선언을 버린다([`StyleDeclaration::parse`] 참고).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StyleDeclaration {
    pub property: String,
    pub value: String,
}

impl StyleDeclaration {
    /// CSS 문자열을 걸러 낸 뒤 남는 선언들로 해석한다.
    ///
    /// 증거가 있는 것만 막는다. 나머지는 통과시킨다 — 목록을 넘겨짚으면 멀쩡한 CSS가
    /// 조용히 사라진다. 렌더확정 근거:
    ///
    /// - `image-rendering`은 속성째 사라진다(문법 도움말은 동작하는 것처럼 서술하지만
    ///   the seed 렌더에는 없다).
    /// - 값이 무효한 선언도 버린다. `틀:다른 뜻`의 `display: @paragraph1=inl@@anchor1=ine@`가
    ///   `display: 5ine`으로 채워지면 the seed는 그 선언을 통째로 버린다.
    pub fn parse(source: &str) -> Vec<StyleDeclaration> {
        source
            .split(';')
            .filter_map(|declaration| {
                let (property, value) = declaration.split_once(':')?;
                let (property, value) = (property.trim(), value.trim());
                (!property.is_empty() && !value.is_empty() && is_supported(property, value)).then(
                    || StyleDeclaration {
                        property: property.to_string(),
                        value: value.to_string(),
                    },
                )
            })
            .collect()
    }

    /// `#!html`의 `style` 속성을 해석한다. 코드를 부를 수 있는 표현이 하나라도 있으면
    /// 속성 전체를 버린다(`None`) — 부분만 지우면 남은 값이 무엇이 될지 장담할 수 없다.
    ///
    /// `#!wiki`의 [`StyleDeclaration::parse`]와 정책이 다르다. `#!wiki`는 `url(`을 받고
    /// (실제 문서가 배경 이미지를 그렇게 쓴다) `image-rendering`·무효한 `display`를 버리는데,
    /// `#!html`은 반대다. 위키 문법이 실어 온 CSS와 사용자가 직접 쓴 HTML은 신뢰 수준이
    /// 달라서 한 함수로 합칠 수 없다 — 합치면 한쪽 렌더가 반드시 어긋난다.
    pub fn parse_html_attribute(source: &str) -> Option<Vec<StyleDeclaration>> {
        // CSS 주석은 위험한 표현을 감출 수 있으므로(`ur/**/l(`) 판정 전에 걷어 낸다.
        // 걷어 낸 값은 판정에만 쓰고, 통과한 선언은 원문 표기 그대로 담는다.
        if calls_code(&strip_comments(source)) {
            return None;
        }
        Some(
            source
                .split(';')
                .filter_map(|declaration| {
                    let (property, value) = declaration.split_once(':')?;
                    let (property, value) = (property.trim(), value.trim());
                    (!property.is_empty() && !value.is_empty()).then(|| StyleDeclaration {
                        property: property.to_string(),
                        value: value.to_string(),
                    })
                })
                .collect(),
        )
    }
}

/// `url(...)`은 외부 요청과 `javascript:`를, `expression(...)`은 옛 IE의 스크립트를 부른다.
fn calls_code(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    ["url(", "expression(", "javascript:", "@import", "behavior:"]
        .iter()
        .any(|pattern| lowered.contains(pattern))
}

fn strip_comments(source: &str) -> String {
    let mut stripped = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("/*") {
        stripped.push_str(&rest[..start]);
        match rest[start..].find("*/") {
            Some(end) => rest = &rest[start + end + 2..],
            // 닫히지 않은 주석 뒤는 전부 주석이다.
            None => return stripped,
        }
    }
    stripped.push_str(rest);
    stripped
}

impl std::fmt::Display for StyleDeclaration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.property, self.value)
    }
}

fn is_supported(property: &str, value: &str) -> bool {
    if has_nested_call(value) {
        return false;
    }
    match property.to_ascii_lowercase().as_str() {
        "image-rendering" => false,
        "display" => is_display_keyword(&value.to_ascii_lowercase()),
        _ => true,
    }
}

/// 함수 호출 안에 또 함수 호출이 있는가.
///
/// 나무위키는 이런 값을 통째로 버린다 — `repeating-linear-gradient(45deg, #1f719a 6%, …)`는
/// 받지만 `linear-gradient(0deg, rgba(255,255,255,.875), …)`는 안 받는다. 렌더에 중첩 호출이
/// 든 선언이 하나도 없다(`hsla(` 44건·`repeating-` 7건은 있어도 함수 안 함수는 0건).
fn has_nested_call(value: &str) -> bool {
    let mut depth = 0usize;
    let mut previous = ' ';
    for character in value.chars() {
        match character {
            '(' => {
                // 여는 괄호 앞이 이름의 일부면 함수 호출이다.
                if depth > 0 && (previous.is_alphanumeric() || previous == '-') {
                    return true;
                }
                depth += 1;
            }
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        previous = character;
    }
    false
}

fn is_display_keyword(value: &str) -> bool {
    matches!(
        value,
        "block"
            | "contents"
            | "flex"
            | "flow-root"
            | "grid"
            | "inline"
            | "inline-block"
            | "inline-flex"
            | "inline-grid"
            | "inline-table"
            | "list-item"
            | "none"
            | "table"
            | "table-caption"
            | "table-cell"
            | "table-column"
            | "table-column-group"
            | "table-footer-group"
            | "table-header-group"
            | "table-row"
            | "table-row-group"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Superscript,
    Subscript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VideoProvider {
    Youtube,
    KakaoTv,
    NicoVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImageAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImageTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImageLayout {
    #[ts(type = "string | null")]
    pub width: Option<Dimension>,
    #[ts(type = "string | null")]
    pub height: Option<Dimension>,
    pub align: Option<ImageAlignment>,
    #[ts(type = "string | null")]
    pub background_color: Option<ColorValue>,
    pub theme: Option<ImageTheme>,
}

/// 문서 링크가 가리키는 곳의 성격. 나무위키는 셋을 다르게 꾸민다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DocumentLinkKind {
    Existing,
    /// 아직 없는 문서. 검색 엔진이 따라가지 않게 한다.
    Missing,
    /// 지금 보고 있는 문서 자신.
    Current,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum RenderInline {
    Text {
        text: String,
    },
    LineBreak,
    Styled {
        style: TextStyle,
        content: Vec<RenderInline>,
    },
    Literal {
        text: String,
    },
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
    ///
    /// 계약에서 빠져 있다 — 생성된 TypeScript 타입에 이 변형이 없고, 직렬화하려 하면
    /// 조용히 나가는 대신 오류가 난다. layout이 하나라도 남기면 그 자리에서 드러난다.
    #[serde(skip)]
    #[ts(skip)]
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
        #[ts(type = "string | null")]
        color: Option<ColorValue>,
    },
    Math {
        formula: String,
    },
    Anchor {
        name: String,
    },
    ClearFix,
    /// `[목차]` 자리. 내용은 [`RenderTree::table_of_contents`]가 소유한다.
    TableOfContents,
    /// `[각주]` 자리와 문서 끝 잔여 각주. 그 시점까지 쌓인 각주를 가리키는
    /// [`RenderTree::footnotes`]의 인덱스다.
    FootnoteSection {
        notes: Vec<u32>,
    },
    /// 감싸는 요소 없이 문단 안에 놓이는 블록들. `#!if`가 표·리스트를 품을 때 쓴다 —
    /// 조건은 내용을 가릴 뿐 자기 요소를 만들지 않는다.
    Blocks {
        blocks: Vec<RenderBlock>,
    },
    /// `{{{#!wiki}}}` — 나무위키에서 인라인이지만 안에 블록을 품는다.
    /// style은 이미 걸러진 선언 목록이다. 비어 있으면 백엔드가 style 속성을 두지 않는다.
    WikiStyle {
        style: Vec<StyleDeclaration>,
        dark_style: Vec<StyleDeclaration>,
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
    /// `{{{#!html}}}` — 화이트리스트를 통과한 것만 담긴다([`HtmlNode`] 참고).
    Html {
        nodes: Vec<HtmlNode>,
    },
    /// 해석하지 못한 매크로. 화면 일치를 위해 원문 표기로 방출한다.
    Unresolved {
        name: String,
        argument: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `#!wiki`와 `#!html`의 style 정책이 갈리는 것은 의도다. 한쪽으로 합치면 반드시
    /// 어느 한쪽 렌더가 어긋나므로, 그 갈림을 여기 고정해 둔다.
    #[test]
    fn wiki_and_html_style_policies_diverge() {
        let background = "background: url(/image.png)";
        // 실제 문서가 `#!wiki`로 배경 이미지를 쓴다 — 받아야 한다.
        assert_eq!(StyleDeclaration::parse(background).len(), 1);
        // 같은 값을 사용자가 직접 쓴 HTML로 넣으면 받지 않는다.
        assert_eq!(StyleDeclaration::parse_html_attribute(background), None);

        let rendering = "image-rendering: pixelated";
        // the seed는 `#!wiki`의 `image-rendering`을 속성째 버린다.
        assert!(StyleDeclaration::parse(rendering).is_empty());
        // `#!html`에는 그런 근거가 없으므로 통과시킨다.
        assert_eq!(
            StyleDeclaration::parse_html_attribute(rendering),
            Some(vec![StyleDeclaration {
                property: "image-rendering".to_string(),
                value: "pixelated".to_string(),
            }])
        );
    }

    /// 위험한 선언 하나가 style 속성 전체를 버리게 한다 — 부분만 지우면 남은 값이
    /// 무엇이 될지 장담할 수 없다.
    #[test]
    fn one_dangerous_declaration_drops_the_whole_attribute() {
        assert_eq!(
            StyleDeclaration::parse_html_attribute("color: red; background: url(//evil)"),
            None
        );
    }
}
