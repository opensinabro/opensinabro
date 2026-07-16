//! 나무위키 동등 마크업 백엔드.
//!
//! 클래스 어휘(wiki-paragraph, wiki-heading, wiki-table, ...)를 나무위키와 같은
//! 체계로 방출하고, 스타일은 동봉 CSS([`stylesheet`])가 담당한다.
//! 듀얼 색상은 CSS 변수(`--wiki-color`)와 `.dark` 클래스로 처리한다.
//!
//! IR 타입마다 대응하는 마크업 래퍼가 [`std::fmt::Display`]를 구현하고,
//! 태그 방출은 스트리밍 태그 라이터([`tag::Tag`])를 거친다 — 속성값 이스케이프가
//! 자동 적용되고 여닫이 짝이 구조적으로 보장되며, 힙 할당 없이 임의의 Formatter로
//! 스트리밍된다.
//!
//! 안전: 모든 텍스트·속성은 이스케이프한다. `#!html` 원문은 sanitizer가 갖춰지기
//! 전까지 이스케이프된 코드 박스로 방출한다(화면 일치보다 안전 우선).

mod tag;

use crate::tag::{escape_text, percent_encode, tag};
use namumark_ast::{
    HorizontalAlignment, ListKind, TableAttribute, TableAttributeScope, VerticalAlignment,
};
use namumark_ir::{
    Color, ColorValue, Dimension, ImageAlignment, ImageLayout, ImageTheme, RenderBackend,
    RenderBlock, RenderInline, RenderTable, RenderTableCell, RenderTableRow, RenderTree,
    RenderedFootnote, TableOfContentsEntry, TextStyle, VideoProvider,
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
struct TreeMarkup<'tree>(&'tree RenderTree);

impl Display for TreeMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut open_heading_levels: Vec<u8> = Vec::new();
        for block in &self.0.blocks {
            if let RenderBlock::Heading { level, .. } = block {
                while open_heading_levels
                    .last()
                    .is_some_and(|open| *open >= *level)
                {
                    open_heading_levels.pop();
                    formatter.write_str("</div>\n")?;
                }
                write!(formatter, "{}", BlockMarkup(block))?;
                formatter.write_str("<div class=\"wiki-heading-content\">")?;
                open_heading_levels.push(*level);
            } else {
                write!(formatter, "{}", BlockMarkup(block))?;
            }
        }
        for _ in open_heading_levels.drain(..) {
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

struct BlockMarkup<'block>(&'block RenderBlock);

impl Display for BlockMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            RenderBlock::Heading {
                level,
                folded,
                number,
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
                        write!(formatter, " {}", InlinesMarkup(content))
                    })
            }
            RenderBlock::Paragraph(inlines) => tag(formatter, "div")?
                .attribute("class", &"wiki-paragraph")?
                .content_line(|formatter| write!(formatter, "{}", InlinesMarkup(inlines))),
            RenderBlock::HorizontalRule => tag(formatter, "hr")?
                .attribute("class", &"wiki-hr")?
                .void_line(),
            RenderBlock::Quote(blocks) => tag(formatter, "blockquote")?
                .attribute("class", &"wiki-quote")?
                .content_line(|formatter| write!(formatter, "{}", BlocksMarkup(blocks))),
            RenderBlock::List { kind, items } => {
                let (tag_name, list_type) = match kind {
                    ListKind::Unordered => ("ul", None),
                    ListKind::Decimal => ("ol", None),
                    ListKind::LowerAlphabet => ("ol", Some("a")),
                    ListKind::UpperAlphabet => ("ol", Some("A")),
                    ListKind::LowerRoman => ("ol", Some("i")),
                    ListKind::UpperRoman => ("ol", Some("I")),
                };
                tag(formatter, tag_name)?
                    .attribute("class", &"wiki-list")?
                    .attribute_if_some(
                        "type",
                        list_type.as_ref().map(|value| value as &dyn Display),
                    )?
                    .content_line(|formatter| {
                        for item in items {
                            tag(formatter, "li")?
                                .attribute_if_some(
                                    "value",
                                    item.start_number
                                        .as_ref()
                                        .map(|value| value as &dyn Display),
                                )?
                                .content(|formatter| {
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
            RenderBlock::CodeBlock { language, source } => tag(formatter, "pre")?
                .attribute("class", &"wiki-code")?
                .content_line(|formatter| {
                    let language_class = language.as_deref().map(LanguageClass);
                    tag(formatter, "code")?
                        .attribute_if_some(
                            "class",
                            language_class.as_ref().map(|value| value as &dyn Display),
                        )?
                        .content(|formatter| write!(formatter, "{}", escape_text(source)))
                }),
            RenderBlock::WikiStyle {
                style,
                dark_style,
                blocks,
            } => tag(formatter, "div")?
                .attribute("class", &"wiki-style")?
                .attribute("style", &style.as_deref().unwrap_or(""))?
                .attribute_if_some(
                    "data-dark-style",
                    dark_style.as_ref().map(|value| value as &dyn Display),
                )?
                .content_line(|formatter| write!(formatter, "{}", BlocksMarkup(blocks))),
            RenderBlock::Folding { summary, blocks } => tag(formatter, "dl")?
                .attribute("class", &"wiki-folding")?
                .content_line(|formatter| {
                    tag(formatter, "dt")?.content(|formatter| {
                        if summary.is_empty() {
                            formatter.write_str("More")
                        } else {
                            write!(formatter, "{}", InlinesMarkup(summary))
                        }
                    })?;
                    tag(formatter, "dd")?
                        .content(|formatter| write!(formatter, "{}", BlocksMarkup(blocks)))
                }),
            RenderBlock::Colored { color, blocks } => tag(formatter, "div")?
                .attribute("class", &"wiki-colored")?
                .attribute("style", &ColorVariables(color))?
                .content_line(|formatter| write!(formatter, "{}", BlocksMarkup(blocks))),
            RenderBlock::Sized { level, blocks } => tag(formatter, "div")?
                .attribute("class", &SizeClass(*level))?
                .content_line(|formatter| write!(formatter, "{}", BlocksMarkup(blocks))),
            RenderBlock::Html(html) => tag(formatter, "pre")?
                .attribute("class", &"wiki-raw-html")?
                .content(|formatter| {
                    tag(formatter, "code")?
                        .content(|formatter| write!(formatter, "{}", escape_text(html)))
                }),
            RenderBlock::TableOfContents { entries } => {
                write!(formatter, "{}", TableOfContentsMarkup(entries))
            }
            RenderBlock::FootnoteSection { notes } => {
                write!(formatter, "{}", FootnoteSectionMarkup(notes))
            }
        }
    }
}

struct LanguageClass<'language>(&'language str);

impl Display for LanguageClass<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "language-{}", self.0)
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
                tag(formatter, "div")?
                    .attribute("class", &"toc-head")?
                    .content(|formatter| formatter.write_str("목차"))?;
                for entry in self.0 {
                    tag(formatter, "div")?
                        .attribute(
                            "class",
                            &format_args!("toc-item toc-indent-{}", entry.depth),
                        )?
                        .content(|formatter| {
                            tag(formatter, "a")?
                                .attribute("href", &format_args!("#s-{}", entry.number))?
                                .content(|formatter| write!(formatter, "{}.", entry.number))?;
                            write!(formatter, " {}", InlinesMarkup(&entry.title))
                        })?;
                }
                Ok(())
            })
    }
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
                if footnote.reference_count <= 1 {
                    tag(formatter, "a")?
                        .attribute("id", &format_args!("fn-{label}"))?
                        .attribute("href", &format_args!("#rfn-{label}-0"))?
                        .content(|formatter| write!(formatter, "[{}]", escape_text(label)))?;
                    formatter.write_char(' ')?;
                } else {
                    tag(formatter, "span")?
                        .attribute("id", &format_args!("fn-{label}"))?
                        .content(|formatter| write!(formatter, "[{}]", escape_text(label)))?;
                    for reference_index in 0..footnote.reference_count {
                        formatter.write_char(' ')?;
                        tag(formatter, "a")?
                            .attribute("href", &format_args!("#rfn-{label}-{reference_index}"))?
                            .content(|formatter| {
                                write!(formatter, "{}.{}", escape_text(label), reference_index + 1)
                            })?;
                    }
                    formatter.write_char(' ')?;
                }
                write!(formatter, "{}", InlinesMarkup(&footnote.content))
            })
    }
}

struct TableMarkup<'table>(&'table RenderTable);

impl Display for TableMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        tag(formatter, "div")?
            .attribute("class", &"wiki-table-wrap")?
            .content_line(|formatter| {
                tag(formatter, "table")?
                    .attribute("class", &"wiki-table")?
                    .content(|formatter| {
                        tag(formatter, "tbody")?.content(|formatter| {
                            if let Some(caption) = &self.0.caption {
                                tag(formatter, "caption")?.content(|formatter| {
                                    write!(formatter, "{}", InlinesMarkup(caption))
                                })?;
                            }
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
        tag(formatter, "td")?
            .attribute_when(cell.column_span > 1, "colspan", &cell.column_span)?
            .attribute_when(cell.row_span > 1, "rowspan", &cell.row_span)?
            .attribute("style", &CellStyle(cell))?
            .content(|formatter| write!(formatter, "{}", BlocksMarkup(&cell.blocks)))
    }
}

// ---- 스타일 값 (Display 합성, 중간 문자열 없음) ----

/// 이 속성이 스타일 속성 문자열을 실제로 방출하는가
fn emits_style(attribute: &TableAttribute) -> bool {
    attribute.value.is_some()
        && matches!(
            attribute.name.as_str(),
            "bgcolor" | "color" | "width" | "height" | "textalign"
        )
}

fn write_table_style(formatter: &mut Formatter<'_>, attribute: &TableAttribute) -> fmt::Result {
    let Some(value) = &attribute.value else {
        return Ok(());
    };
    // 듀얼 색상(`#fff,#000`)은 라이트 값을 쓴다. 다크 모드 값은 후속 과제.
    let value = value.split(',').next().unwrap_or(value);
    match attribute.name.as_str() {
        "bgcolor" => write!(
            formatter,
            " background-color: {};",
            ColorValue::parse(value)
        ),
        "color" => write!(formatter, " color: {};", ColorValue::parse(value)),
        "width" => write!(formatter, " width: {};", Dimension::parse(value)),
        "height" => write!(formatter, " height: {};", Dimension::parse(value)),
        "textalign" => write!(formatter, " text-align: {};", value.trim()),
        _ => Ok(()),
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
        write!(
            formatter,
            "text-align: {};",
            match self.0.horizontal_alignment {
                HorizontalAlignment::Left => "left",
                HorizontalAlignment::Center => "center",
                HorizontalAlignment::Right => "right",
            }
        )?;
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
        for attribute in &self.0.attributes {
            if matches!(
                attribute.scope,
                TableAttributeScope::Cell | TableAttributeScope::Column
            ) {
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
            RenderInline::Literal(text) => tag(formatter, "code")?
                .content(|formatter| write!(formatter, "{}", escape_text(text))),
            RenderInline::Colored { color, content } => tag(formatter, "span")?
                .attribute("class", &"wiki-colored")?
                .attribute("style", &ColorVariables(color))?
                .content(|formatter| write!(formatter, "{}", InlinesMarkup(content))),
            RenderInline::Sized { level, content } => tag(formatter, "span")?
                .attribute("class", &SizeClass(*level))?
                .content(|formatter| write!(formatter, "{}", InlinesMarkup(content))),
            RenderInline::DocumentLink {
                title,
                anchor,
                exists,
                display,
            } => {
                let exists_class = if *exists { "" } else { " not-exist" };
                tag(formatter, "a")?
                    .attribute("class", &format_args!("wiki-link-internal{exists_class}"))?
                    .attribute(
                        "href",
                        &DocumentHref {
                            title,
                            anchor: anchor.as_deref(),
                        },
                    )?
                    .attribute("title", &title)?
                    .content(|formatter| match display {
                        Some(display) => write!(formatter, "{}", InlinesMarkup(display)),
                        None => write!(formatter, "{}", escape_text(title)),
                    })
            }
            RenderInline::ExternalLink { url, display } => {
                let trimmed = url.trim_start();
                let is_javascript = trimmed.len() >= "javascript:".len()
                    && trimmed.as_bytes()[.."javascript:".len()]
                        .eq_ignore_ascii_case(b"javascript:");
                let safe_url = if is_javascript { "#" } else { url.as_str() };
                tag(formatter, "a")?
                    .attribute("class", &"wiki-link-external")?
                    .attribute("href", &safe_url)?
                    .attribute("target", &"_blank")?
                    .attribute("rel", &"noopener nofollow")?
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
                Some(url) => tag(formatter, "span")?
                    .attribute("class", &ImageClass(layout))?
                    .content(|formatter| {
                        tag(formatter, "img")?
                            .attribute("src", &url)?
                            .attribute("alt", &file_name)?
                            .attribute("style", &ImageStyle(layout))?
                            .void()
                    }),
                None => tag(formatter, "a")?
                    .attribute("class", &"wiki-link-internal not-exist")?
                    .attribute("title", &format_args!("파일:{file_name}"))?
                    .content(|formatter| write!(formatter, "파일:{}", escape_text(file_name))),
            },
            RenderInline::FootnoteReference {
                label,
                reference_index,
            } => tag(formatter, "a")?
                .attribute("class", &"wiki-fn-content")?
                .attribute("id", &format_args!("rfn-{label}-{reference_index}"))?
                .attribute("href", &format_args!("#fn-{label}"))?
                .content(|formatter| write!(formatter, "[{}]", escape_text(label))),
            RenderInline::Video {
                provider,
                identifier,
                width,
                height,
            } => tag(formatter, "iframe")?
                .attribute("class", &"wiki-video")?
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
            RenderInline::Ruby { content, ruby } => tag(formatter, "ruby")?.content(|formatter| {
                write!(formatter, "{}", escape_text(content))?;
                tag(formatter, "rp")?.content(|formatter| formatter.write_char('('))?;
                tag(formatter, "rt")?
                    .content(|formatter| write!(formatter, "{}", escape_text(ruby)))?;
                tag(formatter, "rp")?.content(|formatter| formatter.write_char(')'))
            }),
            RenderInline::Math { formula } => tag(formatter, "span")?
                .attribute("class", &"wiki-math")?
                .attribute("data-formula", &formula)?
                .content(|formatter| write!(formatter, "\\({}\\)", escape_text(formula))),
            RenderInline::Anchor { name } => tag(formatter, "a")?
                .attribute("id", &name)?
                .content(|_| Ok(())),
            RenderInline::ClearFix => tag(formatter, "div")?
                .attribute("class", &"wiki-clearfix")?
                .content(|_| Ok(())),
            RenderInline::Html(html) => tag(formatter, "code")?
                .content(|formatter| write!(formatter, "{}", escape_text(html))),
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

impl Display for VideoSource<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.provider {
            VideoProvider::Youtube => write!(
                formatter,
                "https://www.youtube.com/embed/{}",
                percent_encode(self.identifier)
            ),
            VideoProvider::KakaoTv => write!(
                formatter,
                "https://tv.kakao.com/embed/player/cliplink/{}",
                percent_encode(self.identifier)
            ),
            VideoProvider::NicoVideo => write!(
                formatter,
                "https://embed.nicovideo.jp/watch/{}",
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
struct ColorVariables<'color>(&'color Color);

impl Display for ColorVariables<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let dark = self.0.dark.as_ref().unwrap_or(&self.0.light);
        write!(
            formatter,
            "--wiki-color: {}; --wiki-color-dark: {dark};",
            self.0.light
        )
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
        formatter.write_str("wiki-image")?;
        if let Some(align) = self.0.align {
            let align = match align {
                ImageAlignment::Left => "left",
                ImageAlignment::Center => "center",
                ImageAlignment::Right => "right",
            };
            write!(formatter, " wiki-image-align-{align}")?;
        }
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
