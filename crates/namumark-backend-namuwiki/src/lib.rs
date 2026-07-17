//! 나무위키 동등 마크업 백엔드.
//!
//! 클래스 어휘(wiki-paragraph, wiki-heading, wiki-table, ...)를 나무위키와 같은
//! 체계로 방출하고, 스타일은 동봉 CSS([`stylesheet`])가 담당한다.
//! 듀얼 색상은 라이트를 `style`로, 다크를 `data-dark-style`로 낸다 — 나무위키 표기다.
//!
//! IR 타입마다 대응하는 마크업 래퍼가 [`std::fmt::Display`]를 구현하고,
//! 태그 방출은 스트리밍 태그 라이터([`tag::Tag`])를 거친다 — 속성값 이스케이프가
//! 자동 적용되고 여닫이 짝이 구조적으로 보장되며, 힙 할당 없이 임의의 Formatter로
//! 스트리밍된다.
//!
//! 안전: 모든 텍스트·속성은 이스케이프한다. `#!html` 원문만 예외인데, 화이트리스트
//! sanitizer([`sanitize`])를 거쳐 아는 태그·속성만 통과시킨다.

mod sanitize;
mod style;
mod tag;

use crate::style::SupportedStyle;
use crate::tag::{escape_text, percent_encode, percent_encode_anchor, tag};
use namumark_ast::{HorizontalAlignment, ListKind, TableAttributeScope, VerticalAlignment};
use namumark_ir::{
    Color, ColorValue, Dimension, DocumentLinkKind, ImageAlignment, ImageLayout, ImageTheme,
    RenderBackend, RenderBlock, RenderInline, RenderTable, RenderTableAttribute, RenderTableCell,
    RenderTableRow, RenderTree, RenderedFootnote, TableOfContentsEntry, TextStyle, VideoProvider,
};
use std::fmt::{self, Display, Formatter, Write as _};

/// 나무위키 동등 마크업을 문자열로 방출하는 백엔드.
pub struct NamuwikiMarkup;

impl RenderBackend for NamuwikiMarkup {
    type Output = String;

    fn render(&self, tree: &RenderTree) -> String {
        namuwiki_markup(tree).to_string()
    }
}

/// RenderTree를 나무위키 동등 마크업으로 지연 방출하는 Display 어댑터.
pub fn namuwiki_markup(tree: &RenderTree) -> impl Display + '_ {
    TreeMarkup(tree)
}

/// 나무위키 동등 마크업용 동봉 스타일시트.
pub fn stylesheet() -> &'static str {
    include_str!("../assets/namumark.css")
}

/// 문서 전체. 헤딩 콘텐츠 래퍼(`wiki-heading-content`)의 개폐는
/// 형제 블록 순서에 걸친 상태이므로 태그 라이터의 예외로서 문서 래퍼가 수동 관리한다.
///
/// 래퍼는 수준과 무관하게 헤딩마다 닫고 다시 연다 — 나무위키는 하위 문단을 상위 문단
/// 안에 넣지 않는다(렌더확정: the seed의 `wiki-heading-content`는 중첩이 없다).
struct TreeMarkup<'tree>(&'tree RenderTree);

impl Display for TreeMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut heading_content_open = false;
        for block in &self.0.blocks {
            if let RenderBlock::Heading { .. } = block {
                if heading_content_open {
                    formatter.write_str("</div>\n")?;
                }
                write!(formatter, "{}", BlockMarkup(block))?;
                formatter.write_str("<div class=\"wiki-heading-content\">")?;
                heading_content_open = true;
            } else {
                write!(formatter, "{}", BlockMarkup(block))?;
            }
        }
        if heading_content_open {
            formatter.write_str("</div>\n")?;
        }
        if !self.0.categories.is_empty() {
            write!(formatter, "{}", CategoriesMarkup(&self.0.categories))?;
        }
        Ok(())
    }
}

fn heading_tag_name(level: u8) -> &'static str {
    match level.clamp(1, 6) {
        1 => "h1",
        2 => "h2",
        3 => "h3",
        4 => "h4",
        5 => "h5",
        _ => "h6",
    }
}

/// `#!wiki` div 자체. 나무위키에서 이 div는 **언제나 문단 안에** 있다.
/// 표 셀처럼 이미 문단 래퍼가 있는 자리에서는 이것만 쓰고, 그렇지 않은 자리는
/// [`BlockMarkup`]이 문단으로 감싼다.
struct WikiStyleMarkup<'inline>(&'inline RenderInline);

impl Display for WikiStyleMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let RenderInline::WikiStyle {
            style,
            dark_style,
            blocks,
        } = self.0
        else {
            return Ok(());
        };
        // 위키 입력이 CSS로 나가는 자리라 걸러 낸다.
        let style = style.as_deref().map(SupportedStyle);
        let dark_style = dark_style.as_deref().map(SupportedStyle);
        // `#!wiki`는 style만 실은 맨 div다 — 나무위키는 여기에 클래스를 주지 않는다.
        tag(formatter, "div")?
            .attribute_if_some(
                "style",
                style
                    .as_ref()
                    .filter(|style| !style.is_empty())
                    .map(|style| style as &dyn Display),
            )?
            .attribute_if_some(
                "data-dark-style",
                dark_style
                    .as_ref()
                    .filter(|style| !style.is_empty())
                    .map(|style| style as &dyn Display),
            )?
            .content(|formatter| write_wiki_style_content(formatter, blocks))
    }
}

/// `#!wiki` 안의 내용. 나무위키는 여기서 문단 래퍼를 만들지 않고, 앞 블록에서 이어지는
/// 문단 앞에 개행 하나를 둔다.
///
/// 렌더확정: 리스트 뒤 빈 줄 다음 문단이 `</ul><br><br>문서 내에…`(문단 앞 `<br>` 하나 +
/// 빈 줄이 문단에 남긴 줄바꿈 하나)인 반면, 문단 뒤에 바로 붙는 표 앞에는 `<br>`이 없다.
fn write_wiki_style_content(formatter: &mut Formatter<'_>, blocks: &[RenderBlock]) -> fmt::Result {
    for (index, block) in blocks.iter().enumerate() {
        match block {
            RenderBlock::Paragraph(inlines) => {
                if index > 0 {
                    formatter.write_str("<br>")?;
                }
                write!(formatter, "{}", InlinesMarkup(inlines))?;
            }
            block => write!(formatter, "{}", BlockMarkup(block))?,
        }
    }
    Ok(())
}

struct BlockMarkup<'block>(&'block RenderBlock);

impl Display for BlockMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            RenderBlock::Heading {
                level,
                folded,
                number,
                anchor,
                content,
            } => {
                let folded_class = if *folded { " wiki-heading-folded" } else { "" };
                tag(formatter, heading_tag_name(*level))?
                    .attribute("class", &format_args!("wiki-heading{folded_class}"))?
                    .content_line(|formatter| {
                        tag(formatter, "a")?
                            .attribute("id", &format_args!("s-{number}"))?
                            .attribute("href", &"#toc")?
                            .content(|formatter| write!(formatter, "{number}."))?;
                        formatter.write_str(" ")?;
                        // 제목 글자로 건 문단명 앵커. `[[#개요]]`가 이걸 가리킨다.
                        tag(formatter, "span")?
                            .attribute("id", &anchor)?
                            .content(|formatter| write!(formatter, "{}", InlinesMarkup(content)))
                    })
            }
            RenderBlock::Paragraph(inlines) => tag(formatter, "div")?
                .attribute("class", &"wiki-paragraph")?
                .content_line(|formatter| write!(formatter, "{}", InlinesMarkup(inlines))),
            // 나무위키의 `<hr>`은 맨 태그다.
            RenderBlock::HorizontalRule => tag(formatter, "hr")?.void_line(),
            RenderBlock::Quote(blocks) => tag(formatter, "blockquote")?
                .attribute("class", &"wiki-quote")?
                .content_line(|formatter| write!(formatter, "{}", BlocksMarkup(blocks))),
            // 나무위키는 순서 리스트의 모양을 클래스로 주고 시작 번호를 `start`로 준다
            // (렌더확정: `<ol class='wiki-list wiki-list-upper-roman' start=11>`, `<li>`는
            // 속성이 없다). 첫 항목의 재지정 번호가 곧 리스트의 시작 번호다.
            RenderBlock::List { kind, items } => {
                let ordered = !matches!(kind, ListKind::Unordered);
                let start = items
                    .first()
                    .and_then(|item| item.start_number)
                    .unwrap_or(1);
                tag(formatter, if ordered { "ol" } else { "ul" })?
                    .attribute("class", &ListClass(*kind))?
                    .attribute_when(ordered, "start", &start)?
                    .content_line(|formatter| {
                        for item in items {
                            tag(formatter, "li")?.content(|formatter| {
                                write!(formatter, "{}", BlocksMarkup(&item.blocks))
                            })?;
                        }
                        Ok(())
                    })
            }
            RenderBlock::Indent(blocks) => tag(formatter, "div")?
                .attribute("class", &"wiki-indent")?
                .content_line(|formatter| write!(formatter, "{}", BlocksMarkup(blocks))),
            RenderBlock::Table(table) => write!(formatter, "{}", TableMarkup(table)),
        }
    }
}

struct BlocksMarkup<'blocks>(&'blocks [RenderBlock]);

impl Display for BlocksMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for block in self.0 {
            write!(formatter, "{}", BlockMarkup(block))?;
        }
        Ok(())
    }
}

struct TableOfContentsMarkup<'entries>(&'entries [TableOfContentsEntry]);

impl Display for TableOfContentsMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        tag(formatter, "div")?
            .attribute("class", &"wiki-macro-toc")?
            .attribute("id", &"toc")?
            .content_line(|formatter| {
                // 나무위키 목차는 접을 수 있고, 접힘 상태 표시는 빈 summary가 맡는다.
                tag(formatter, "details")?
                    .flag("open")?
                    .content(|formatter| {
                        tag(formatter, "summary")?.content(|_| Ok(()))?;
                        if let Some(first) = self.0.first() {
                            write_table_of_contents_level(formatter, self.0, first.depth)?;
                        }
                        Ok(())
                    })
            })
    }
}

/// 목차 항목을 깊이별 `toc-indent` 컨테이너로 중첩해 방출한다.
///
/// 하위 항목 묶음은 상위 항목의 **형제**로 오지, 그 안에 들어가지 않는다.
/// 슬라이스는 모두 `depth` 이상이며 첫 항목이 정확히 `depth`다.
fn write_table_of_contents_level(
    formatter: &mut Formatter<'_>,
    entries: &[TableOfContentsEntry],
    depth: u8,
) -> fmt::Result {
    tag(formatter, "div")?
        .attribute("class", &"toc-indent")?
        .content(|formatter| {
            let mut index = 0;
            while index < entries.len() {
                let entry = &entries[index];
                tag(formatter, "span")?
                    .attribute("class", &"toc-item")?
                    .content(|formatter| {
                        tag(formatter, "a")?
                            .attribute("href", &format_args!("#s-{}", entry.number))?
                            .content(|formatter| formatter.write_str(&entry.number))?;
                        write!(formatter, ". {}", InlinesMarkup(&entry.title))
                    })?;
                index += 1;

                let children_start = index;
                while index < entries.len() && entries[index].depth > depth {
                    index += 1;
                }
                if children_start < index {
                    write_table_of_contents_level(
                        formatter,
                        &entries[children_start..index],
                        depth + 1,
                    )?;
                }
            }
            Ok(())
        })
}

struct FootnoteSectionMarkup<'notes>(&'notes [RenderedFootnote]);

impl Display for FootnoteSectionMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        tag(formatter, "div")?
            .attribute("class", &"wiki-macro-footnote")?
            .content_line(|formatter| {
                for footnote in self.0 {
                    write!(formatter, "{}", FootnoteMarkup(footnote))?;
                }
                Ok(())
            })
    }
}

struct FootnoteMarkup<'footnote>(&'footnote RenderedFootnote);

impl Display for FootnoteMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let footnote = self.0;
        let label = footnote.label.as_str();
        tag(formatter, "span")?
            .attribute("class", &"footnote-list")?
            .content(|formatter| {
                // 각주 자리로 돌아오는 앵커는 라벨 옆 빈 span이 갖는다.
                tag(formatter, "span")?
                    .attribute("id", &format_args!("fn-{label}"))?
                    .content(|_| Ok(()))?;
                match footnote.reference_numbers.as_slice() {
                    // 한 번만 참조했으면 라벨 자체가 되돌아가는 링크다.
                    [number] => {
                        tag(formatter, "a")?
                            .attribute("href", &format_args!("#rfn-{number}"))?
                            .content(|formatter| write!(formatter, "[{}]", escape_text(label)))?;
                    }
                    // 여러 번 참조했으면 라벨은 글자로 두고 참조마다 돌아가는 링크를 단다.
                    // 링크 글자는 `첫참조번호.순번`이다(렌더확정: `[A]`가 13·14면 13.1·13.2).
                    numbers => {
                        write!(formatter, "[{}]", escape_text(label))?;
                        let first = numbers.first().copied().unwrap_or_default();
                        for (index, number) in numbers.iter().enumerate() {
                            formatter.write_char(' ')?;
                            tag(formatter, "a")?
                                .attribute("href", &format_args!("#rfn-{number}"))?
                                .content(|formatter| {
                                    tag(formatter, "sup")?.content(|formatter| {
                                        write!(formatter, "{first}.{}", index + 1)
                                    })
                                })?;
                        }
                    }
                }
                formatter.write_char(' ')?;
                write!(formatter, "{}", InlinesMarkup(&footnote.content))
            })
    }
}

/// 빈 셀이 갖는 문단.
const EMPTY_PARAGRAPH: RenderBlock = RenderBlock::Paragraph(Vec::new());

struct TableMarkup<'table>(&'table RenderTable);

impl Display for TableMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let table = self.0;
        let has_width = table_attributes(table).any(|attribute| attribute.name == "width");
        let has_table_style = table_attributes(table).any(emits_table_style);
        tag(formatter, "div")?
            .attribute("class", &TableWrapClass(table))?
            .attribute_when(has_width, "style", &TableWrapStyle(table))?
            .content_line(|formatter| {
                tag(formatter, "table")?
                    .attribute("class", &"wiki-table")?
                    .attribute_when(has_table_style, "style", &TableStyle(table))?
                    .content(|formatter| {
                        // 캡션은 `<tbody>` 앞이다 — HTML이 그렇게 정해 두었고 the seed도 그렇다.
                        if let Some(caption) = &self.0.caption {
                            tag(formatter, "caption")?.content(|formatter| {
                                write!(formatter, "{}", InlinesMarkup(caption))
                            })?;
                        }
                        tag(formatter, "tbody")?.content(|formatter| {
                            for row in &self.0.rows {
                                write!(formatter, "{}", TableRowMarkup(row))?;
                            }
                            Ok(())
                        })
                    })
            })
    }
}

struct TableRowMarkup<'row>(&'row RenderTableRow);

impl Display for TableRowMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let has_row_style = self.0.cells.iter().any(|cell| {
            cell.attributes.iter().any(|attribute| {
                attribute.scope == TableAttributeScope::Row && emits_style(attribute)
            })
        });
        tag(formatter, "tr")?
            .attribute("class", &"wiki-table-tr")?
            .attribute_when(has_row_style, "style", &RowStyle(self.0))?
            .content(|formatter| {
                for cell in &self.0.cells {
                    write!(formatter, "{}", TableCellMarkup(cell))?;
                }
                Ok(())
            })
    }
}

struct TableCellMarkup<'cell>(&'cell RenderTableCell);

impl Display for TableCellMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let cell = self.0;
        // `<nopad>`는 style이 아니라 클래스로 나간다.
        let nopadding = cell.attributes.iter().any(|attribute| {
            attribute.scope == TableAttributeScope::Cell && attribute.name == "nopad"
        });
        tag(formatter, "td")?
            .attribute_when(nopadding, "class", &"wiki-table-nopadding")?
            .attribute_if_some(
                "colspan",
                cell.column_span.as_ref().map(|span| span as &dyn Display),
            )?
            .attribute_if_some(
                "rowspan",
                cell.row_span.as_ref().map(|span| span as &dyn Display),
            )?
            .attribute("style", &CellStyle(cell))?
            // 셀 안은 블록을 그대로 낸다. 내용이 문단 하나뿐인 셀이 언제나
            // `<div class='wiki-paragraph'>내용</div>`인 것은 그 결과일 뿐이다 —
            // 문단과 리스트가 같이 있으면 the seed도 둘을 나란히 놓는다.
            // 빈 셀도 빈 문단 하나는 갖는다.
            .content(|formatter| match cell.blocks.as_slice() {
                [] => write!(formatter, "{}", BlockMarkup(&EMPTY_PARAGRAPH)),
                blocks => write!(formatter, "{}", BlocksMarkup(blocks)),
            })
    }
}

// ---- 스타일 값 (Display 합성, 중간 문자열 없음) ----

/// 이 속성이 스타일 속성 문자열을 실제로 방출하는가
fn emits_style(attribute: &RenderTableAttribute) -> bool {
    attribute.value.is_some()
        && matches!(
            attribute.name.as_str(),
            "bgcolor" | "color" | "width" | "height" | "textalign"
        )
}

fn write_table_style(
    formatter: &mut Formatter<'_>,
    attribute: &RenderTableAttribute,
) -> fmt::Result {
    let Some(value) = &attribute.value else {
        return Ok(());
    };
    // 듀얼 색상(`#fff,#000`)은 라이트 값을 쓴다. 다크 모드 값은 후속 과제.
    let value = value.split(',').next().unwrap_or(value);
    // 색이 아닌 값이 들어온 선언은 통째로 버린다 — 나무위키가 그렇게 한다.
    match attribute.name.as_str() {
        "bgcolor" => match ColorValue::parse(value) {
            Some(color) => write!(formatter, " background-color: {color};"),
            None => Ok(()),
        },
        "color" => match ColorValue::parse(value) {
            Some(color) => write!(formatter, " color: {color};"),
            None => Ok(()),
        },
        "width" => write!(formatter, " width: {};", Dimension::parse(value)),
        "height" => write!(formatter, " height: {};", Dimension::parse(value)),
        "textalign" => write!(formatter, " text-align: {};", value.trim()),
        _ => Ok(()),
    }
}

// ---- 표 전체(`table`) 스코프 ----
//
// 나무위키는 표 스코프 속성을 두 군데로 나눠 싣는다. 정렬은 감싸는 div의 클래스로,
// 너비는 div의 style로 가고(이때 표 자신은 100%가 된다), 나머지 색·테두리는 table의 style로 간다.

/// 표의 모든 셀에 흩어져 있는 표 스코프 속성.
fn table_attributes(table: &RenderTable) -> impl Iterator<Item = &RenderTableAttribute> {
    table
        .rows
        .iter()
        .flat_map(|row| row.cells.iter())
        .flat_map(|cell| cell.attributes.iter())
        .filter(|attribute| attribute.scope == TableAttributeScope::Table)
}

fn emits_table_style(attribute: &RenderTableAttribute) -> bool {
    attribute.value.is_some()
        && matches!(
            attribute.name.as_str(),
            "bgcolor" | "color" | "bordercolor" | "height" | "textalign" | "width"
        )
}

struct TableWrapClass<'table>(&'table RenderTable);

impl Display for TableWrapClass<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("wiki-table-wrap")?;
        // 같은 속성이 여러 번 지정되면 마지막 것이 이긴다.
        let alignment = table_attributes(self.0)
            .filter(|attribute| attribute.name == "align")
            .filter_map(|attribute| attribute.value.as_deref())
            .last();
        match alignment.map(str::trim) {
            // 왼쪽은 기본값이라 클래스를 붙이지 않는다.
            Some("center") => formatter.write_str(" table-center"),
            Some("right") => formatter.write_str(" table-right"),
            _ => Ok(()),
        }
    }
}

struct TableWrapStyle<'table>(&'table RenderTable);

impl Display for TableWrapStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for attribute in table_attributes(self.0) {
            if attribute.name == "width"
                && let Some(value) = &attribute.value
            {
                write!(formatter, "width: {};", Dimension::parse(value))?;
            }
        }
        Ok(())
    }
}

struct TableStyle<'table>(&'table RenderTable);

impl Display for TableStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for attribute in table_attributes(self.0) {
            let Some(value) = &attribute.value else {
                continue;
            };
            // 듀얼 색상(`#fff,#000`)은 라이트 값을 쓴다. 다크 모드 값은 후속 과제.
            let value = value.split(',').next().unwrap_or(value);
            match attribute.name.as_str() {
                // 너비가 지정되면 감싸는 div가 그 폭을 갖고 표는 그 안을 채운다.
                "width" => write!(formatter, " width: 100%;",)?,
                "bgcolor" => {
                    if let Some(color) = ColorValue::parse(value) {
                        write!(formatter, " background-color: {color};")?;
                    }
                }
                "color" => {
                    if let Some(color) = ColorValue::parse(value) {
                        write!(formatter, " color: {color};")?;
                    }
                }
                "bordercolor" => {
                    if let Some(color) = ColorValue::parse(value) {
                        write!(formatter, " border: 2px solid {color};")?;
                    }
                }
                "height" => write!(formatter, " height: {};", Dimension::parse(value))?,
                "textalign" => write!(formatter, " text-align: {};", value.trim())?,
                _ => {}
            }
        }
        Ok(())
    }
}

struct RowStyle<'row>(&'row RenderTableRow);

impl Display for RowStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for cell in &self.0.cells {
            for attribute in &cell.attributes {
                if attribute.scope == TableAttributeScope::Row {
                    write_table_style(formatter, attribute)?;
                }
            }
        }
        Ok(())
    }
}

struct CellStyle<'cell>(&'cell RenderTableCell);

impl Display for CellStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(alignment) = self.0.horizontal_alignment {
            write!(
                formatter,
                "text-align: {};",
                match alignment {
                    HorizontalAlignment::Left => "left",
                    HorizontalAlignment::Center => "center",
                    HorizontalAlignment::Right => "right",
                }
            )?;
        }
        if let Some(vertical_alignment) = self.0.vertical_alignment {
            write!(
                formatter,
                " vertical-align: {};",
                match vertical_alignment {
                    VerticalAlignment::Top => "top",
                    VerticalAlignment::Bottom => "bottom",
                }
            )?;
        }
        // 같은 속성을 셀과 열이 함께 주면 셀이 이긴다(문법 도움말·렌더확정:
        // `bgcolor > colbgcolor > rowbgcolor > tablebgcolor`). 둘 다 실으면 나중 선언이
        // 이겨 열 색이 셀 색을 덮는다.
        for attribute in &self.0.attributes {
            let overridden = matches!(attribute.scope, TableAttributeScope::Column { .. })
                && self.0.attributes.iter().any(|other| {
                    other.scope == TableAttributeScope::Cell && other.name == attribute.name
                });
            if !overridden
                && matches!(
                    attribute.scope,
                    TableAttributeScope::Cell | TableAttributeScope::Column { .. }
                )
            {
                write_table_style(formatter, attribute)?;
            }
        }
        Ok(())
    }
}

struct InlinesMarkup<'inlines>(&'inlines [RenderInline]);

impl Display for InlinesMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for inline in self.0 {
            write!(formatter, "{}", InlineMarkup(inline))?;
        }
        Ok(())
    }
}

struct InlineMarkup<'inline>(&'inline RenderInline);

impl Display for InlineMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            RenderInline::Text(text) => write!(formatter, "{}", escape_text(text)),
            RenderInline::LineBreak => tag(formatter, "br")?.void(),
            RenderInline::Styled { style, content } => {
                let tag_name = match style {
                    TextStyle::Bold => "strong",
                    TextStyle::Italic => "em",
                    TextStyle::Strikethrough => "del",
                    TextStyle::Underline => "u",
                    TextStyle::Superscript => "sup",
                    TextStyle::Subscript => "sub",
                };
                tag(formatter, tag_name)?
                    .content(|formatter| write!(formatter, "{}", InlinesMarkup(content)))
            }
            // 여러 줄 리터럴은 `<pre><code>`다(렌더확정: 각주 안 `{{{|| … ⏎ … ||}}}`이
            // the seed에서 `<pre><code>…</code></pre>`). 한 줄 리터럴은 인라인 `<code>`뿐이다.
            RenderInline::Literal(text) if text.contains('\n') => tag(formatter, "pre")?
                .content(|formatter| {
                    tag(formatter, "code")?
                        .content(|formatter| write!(formatter, "{}", escape_text(text)))
                }),
            RenderInline::Literal(text) => tag(formatter, "code")?
                .content(|formatter| write!(formatter, "{}", escape_text(text))),
            RenderInline::Colored { color, content } => tag(formatter, "span")?
                .attribute("style", &ColorStyle(color))?
                .attribute("data-dark-style", &DarkColorStyle(color))?
                .content(|formatter| write!(formatter, "{}", InlinesMarkup(content))),
            RenderInline::Sized { level, content } => tag(formatter, "span")?
                .attribute("class", &SizeClass(*level))?
                .content(|formatter| write!(formatter, "{}", InlinesMarkup(content))),
            RenderInline::DocumentLink {
                title,
                anchor,
                kind,
                display,
            } => {
                tag(formatter, "a")?
                    .attribute("class", &DocumentLinkClass(*kind))?
                    .attribute(
                        "href",
                        &DocumentHref {
                            title,
                            anchor: anchor.as_deref(),
                        },
                    )?
                    // 없는 문서로 가는 링크는 검색 엔진이 따라가지 않게 한다.
                    .attribute_when(*kind == DocumentLinkKind::Missing, "rel", &"nofollow")?
                    .attribute("title", &title)?
                    .content(|formatter| write!(formatter, "{}", InlinesMarkup(display)))
            }
            RenderInline::ExternalLink { url, display } => {
                let trimmed = url.trim_start();
                let is_javascript = trimmed.len() >= "javascript:".len()
                    && trimmed.as_bytes()[.."javascript:".len()]
                        .eq_ignore_ascii_case(b"javascript:");
                let safe_url = if is_javascript { "#" } else { url.as_str() };
                // 이미지를 감싼 외부 링크는 클래스가 따로다(렌더확정).
                let is_image_link =
                    matches!(display.as_deref(), Some([RenderInline::Image { .. }]));
                tag(formatter, "a")?
                    .attribute_when(!is_image_link, "class", &"wiki-link-external")?
                    .attribute_when(is_image_link, "class", &"wiki-link-external-image")?
                    .attribute("href", &safe_url)?
                    .attribute("target", &"_blank")?
                    // `ugc`(user generated content)까지 붙이는 것이 나무위키 표기다.
                    .attribute("rel", &"nofollow noopener ugc")?
                    // 툴팁에는 주소만 싣고 `#` 뒤 조각은 뺀다(렌더확정: href는
                    // `…?rev=1939&noredirect=1#s-`인데 title은 `…?rev=1939&noredirect=1`이다).
                    .attribute(
                        "title",
                        &url.split_once('#')
                            .map_or(url.as_str(), |(address, _)| address),
                    )?
                    .content(|formatter| match display {
                        Some(display) => write!(formatter, "{}", InlinesMarkup(display)),
                        None => write!(formatter, "{}", escape_text(url)),
                    })
            }
            RenderInline::Image {
                file_name,
                url,
                layout,
            } => match url {
                // 나무위키는 이미지를 두 겹의 span으로 감싼다. 바깥이 크기·정렬을 잡고,
                // 안쪽 wrapper와 img는 그 안을 100%로 채운다.
                Some(url) => tag(formatter, "span")?
                    .attribute("class", &ImageClass(layout))?
                    .attribute("style", &ImageStyle(layout))?
                    .content(|formatter| {
                        tag(formatter, "span")?
                            .attribute("class", &"wiki-image-wrapper")?
                            .attribute("style", &"width: 100%;")?
                            .content(|formatter| {
                                tag(formatter, "img")?
                                    .attribute("width", &"100%")?
                                    .attribute("src", &url)?
                                    .attribute("alt", &format_args!("파일:{file_name}"))?
                                    .void()
                            })
                    }),
                // 없는 파일은 그 파일 문서로 가는 없는 문서 링크가 된다.
                None => tag(formatter, "a")?
                    .attribute("class", &DocumentLinkClass(DocumentLinkKind::Missing))?
                    .attribute(
                        "href",
                        &DocumentHref {
                            title: &format!("파일:{file_name}"),
                            anchor: None,
                        },
                    )?
                    .attribute("rel", &"nofollow")?
                    .attribute("title", &format_args!("파일:{file_name}"))?
                    .content(|formatter| write!(formatter, "파일:{}", escape_text(file_name))),
            },
            RenderInline::FootnoteReference {
                label,
                number,
                tooltip,
            } => tag(formatter, "a")?
                .attribute("class", &"wiki-fn-content")?
                // 각주 내용을 툴팁으로 띄운다.
                .attribute("title", &tooltip)?
                .attribute(
                    "href",
                    &format_args!("#fn-{}", percent_encode_anchor(label)),
                )?
                .content(|formatter| {
                    // 본문 복귀 앵커(`#rfn-N`)는 링크 안쪽 빈 span이 갖는다.
                    tag(formatter, "span")?
                        .attribute("id", &format_args!("rfn-{number}"))?
                        .content(|_| Ok(()))?;
                    write!(formatter, "[{}]", escape_text(label))
                }),
            RenderInline::Video {
                provider,
                identifier,
                width,
                height,
            } => tag(formatter, "iframe")?
                .attribute("class", &"wiki-media")?
                .attribute(
                    "src",
                    &VideoSource {
                        provider: *provider,
                        identifier,
                    },
                )?
                .attribute("width", &width.as_deref().unwrap_or("640"))?
                .attribute("height", &height.as_deref().unwrap_or("360"))?
                .attribute("frameborder", &"0")?
                .flag("allowfullscreen")?
                .attribute("loading", &"lazy")?
                .content(|_| Ok(())),
            RenderInline::Ruby {
                content,
                ruby,
                color,
            } => tag(formatter, "ruby")?.content(|formatter| {
                write!(formatter, "{}", escape_text(content))?;
                tag(formatter, "rp")?.content(|formatter| formatter.write_char('('))?;
                tag(formatter, "rt")?.content(|formatter| match color {
                    // 루비 글자색은 `<rt>` 안 span이 갖는다.
                    Some(color) => tag(formatter, "span")?
                        .attribute("style", &format_args!("color:{color}"))?
                        .content(|formatter| write!(formatter, "{}", escape_text(ruby))),
                    None => write!(formatter, "{}", escape_text(ruby)),
                })?;
                tag(formatter, "rp")?.content(|formatter| formatter.write_char(')'))
            }),
            RenderInline::Math { formula } => tag(formatter, "span")?
                .attribute("class", &"wiki-math")?
                .attribute("data-formula", &formula)?
                .content(|formatter| write!(formatter, "\\({}\\)", escape_text(formula))),
            RenderInline::Anchor { name } => tag(formatter, "a")?
                .attribute("id", &name)?
                .content(|_| Ok(())),
            RenderInline::WikiStyle { .. } => write!(formatter, "{}", WikiStyleMarkup(self.0)),
            RenderInline::Blocks(blocks) => write!(formatter, "{}", BlocksMarkup(blocks)),
            RenderInline::TableOfContents { entries } => {
                write!(formatter, "{}", TableOfContentsMarkup(entries))
            }
            RenderInline::FootnoteSection { notes } => {
                write!(formatter, "{}", FootnoteSectionMarkup(notes))
            }
            RenderInline::Folding { summary, blocks } => tag(formatter, "details")?
                .attribute("class", &"wiki-folding")?
                .content(|formatter| {
                    tag(formatter, "summary")?.content(|formatter| {
                        if summary.is_empty() {
                            formatter.write_str("More")
                        } else {
                            write!(formatter, "{}", escape_text(summary))
                        }
                    })?;
                    // 접힌 내용도 `#!wiki`처럼 문단 래퍼를 두지 않는 컨테이너다
                    // (렌더확정: `<details class='wiki-folding'>…<div><div style='margin:…'>`).
                    tag(formatter, "div")?
                        .content(|formatter| write_wiki_style_content(formatter, blocks))
                }),
            // `<pre>`는 맨 태그다. `#!syntax`로 언어를 준 코드만 강조기 표식을 단다.
            RenderInline::CodeBlock { language, source } => {
                tag(formatter, "pre")?.content(|formatter| {
                    tag(formatter, "code")?
                        .attribute_when(language.is_some(), "class", &"hljs")?
                        .attribute_if_some(
                            "data-language",
                            language.as_ref().map(|value| value as &dyn Display),
                        )?
                        .content(|formatter| write!(formatter, "{}", escape_text(source)))
                })
            }
            // 원시 HTML은 sanitizer를 거친 뒤 그대로 나간다.
            RenderInline::Html(html) => formatter.write_str(&sanitize::sanitize(html)),
            RenderInline::ClearFix => tag(formatter, "div")?
                .attribute("class", &"wiki-clearfix")?
                .content(|_| Ok(())),
            RenderInline::Unresolved { name, argument } => match argument {
                Some(argument) => write!(
                    formatter,
                    "{}",
                    escape_text(format_args!("[{name}({argument})]"))
                ),
                None => write!(formatter, "{}", escape_text(format_args!("[{name}]"))),
            },
            RenderInline::Footnote { .. } => {
                debug_assert!(false, "layout 이후에는 Footnote 인라인이 남지 않아야 한다");
                Ok(())
            }
        }
    }
}

struct ListClass(ListKind);

impl Display for ListClass {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("wiki-list")?;
        formatter.write_str(match self.0 {
            ListKind::Unordered => return Ok(()),
            // 십진 리스트는 꼬리표가 없어 클래스가 두 번 나온다 — the seed 표기 그대로다.
            ListKind::Decimal => " wiki-list",
            ListKind::LowerAlphabet => " wiki-list-alpha",
            ListKind::UpperAlphabet => " wiki-list-upper-alpha",
            ListKind::LowerRoman => " wiki-list-roman",
            ListKind::UpperRoman => " wiki-list-upper-roman",
        })
    }
}

struct DocumentLinkClass(DocumentLinkKind);

impl Display for DocumentLinkClass {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.0 {
            DocumentLinkKind::Existing => "wiki-link-internal",
            DocumentLinkKind::Missing => "wiki-link-internal not-exist",
            DocumentLinkKind::Current => "wiki-self-link",
        })
    }
}

struct DocumentHref<'link> {
    title: &'link str,
    anchor: Option<&'link str>,
}

impl Display for DocumentHref<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if !self.title.is_empty() {
            write!(formatter, "/w/{}", percent_encode(self.title))?;
        }
        if let Some(anchor) = self.anchor {
            write!(formatter, "#{}", percent_encode(anchor))?;
        }
        Ok(())
    }
}

struct VideoSource<'video> {
    provider: VideoProvider,
    identifier: &'video str,
}

/// 임베드 주소는 프로토콜 상대(`//`)다 — 나무위키가 파일이든 동영상이든 그렇게 낸다
/// (렌더확정: `//www.youtube.com/embed/jNQXAC9IVRw`, 이미지도 `//file.alphawiki.org/…`).
impl Display for VideoSource<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.provider {
            VideoProvider::Youtube => write!(
                formatter,
                "//www.youtube.com/embed/{}",
                percent_encode(self.identifier)
            ),
            VideoProvider::KakaoTv => write!(
                formatter,
                "//tv.kakao.com/embed/player/cliplink/{}",
                percent_encode(self.identifier)
            ),
            VideoProvider::NicoVideo => write!(
                formatter,
                "//embed.nicovideo.jp/watch/{}",
                percent_encode(self.identifier)
            ),
        }
    }
}

struct CategoriesMarkup<'categories>(&'categories [String]);

impl Display for CategoriesMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        tag(formatter, "div")?
            .attribute("class", &"wiki-categories")?
            .content_line(|formatter| {
                formatter.write_str("분류:")?;
                for (index, category) in self.0.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(" | ")?;
                    }
                    formatter.write_char(' ')?;
                    tag(formatter, "a")?
                        .attribute(
                            "href",
                            &format_args!("/w/%EB%B6%84%EB%A5%98%3A{}", percent_encode(category)),
                        )?
                        .content(|formatter| write!(formatter, "{}", escape_text(category)))?;
                }
                Ok(())
            })
    }
}

/// `--wiki-color`/`--wiki-color-dark` CSS 변수 값. 이스케이프는 속성 방출부가 담당한다.
/// 라이트 색상은 `style`로, 다크 색상은 `data-dark-style`로 나간다 — 나무위키 표기다.
/// 다크를 따로 주지 않은 색도 나무위키는 같은 값으로 채운다.
struct ColorStyle<'color>(&'color Color);

impl Display for ColorStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "color:{}", self.0.light)
    }
}

struct DarkColorStyle<'color>(&'color Color);

impl Display for DarkColorStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let dark = self.0.dark.as_ref().unwrap_or(&self.0.light);
        write!(formatter, "color:{dark};")
    }
}

struct SizeClass(i8);

impl Display for SizeClass {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.0 >= 0 {
            write!(formatter, "wiki-size-up-{}", self.0)
        } else {
            write!(formatter, "wiki-size-down-{}", -self.0)
        }
    }
}

struct ImageClass<'layout>(&'layout ImageLayout);

impl Display for ImageClass<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        // 정렬을 지정하지 않은 이미지도 `normal`로 명시한다.
        let align = match self.0.align {
            Some(ImageAlignment::Left) => "left",
            Some(ImageAlignment::Center) => "center",
            Some(ImageAlignment::Right) => "right",
            None => "normal",
        };
        write!(formatter, "wiki-image-align-{align}")?;
        if let Some(theme) = self.0.theme {
            let theme = match theme {
                ImageTheme::Light => "light",
                ImageTheme::Dark => "dark",
            };
            write!(formatter, " wiki-image-theme-{theme}")?;
        }
        Ok(())
    }
}

struct ImageStyle<'layout>(&'layout ImageLayout);

impl Display for ImageStyle<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(width) = &self.0.width {
            write!(formatter, "width: {width};")?;
        }
        if let Some(height) = &self.0.height {
            write!(formatter, "height: {height};")?;
        }
        if let Some(background_color) = &self.0.background_color {
            write!(formatter, "background-color: {background_color};")?;
        }
        Ok(())
    }
}
