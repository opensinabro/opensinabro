//! 나무위키 동등 마크업 백엔드.
//!
//! 클래스 어휘(wiki-paragraph, wiki-heading, wiki-table, ...)를 나무위키와 같은
//! 체계로 방출하고, 스타일은 동봉 CSS([`stylesheet`])가 담당한다.
//! 듀얼 색상은 CSS 변수(`--wiki-color`)와 `.dark` 클래스로 처리한다.
//!
//! IR 타입마다 대응하는 마크업 래퍼가 [`std::fmt::Display`]를 구현한다
//! ([`BlockMarkup`], [`InlineMarkup`], [`TableMarkup`], ...). 문서 래퍼는
//! 형제 블록에 걸치는 상태(헤딩 콘텐츠 래퍼의 개폐)만 소유하고 나머지는 전부
//! 타입별 Display의 합성이다. 따라서 중간 문자열 없이 임의의 Formatter로
//! 스트리밍된다.
//!
//! 안전: 모든 텍스트·속성은 이스케이프한다. `#!html` 원문은 sanitizer가 갖춰지기
//! 전까지 이스케이프된 코드 박스로 방출한다(화면 일치보다 안전 우선).

use namumark_ast::{HorizontalAlignment, ListKind, TableAttributeScope, VerticalAlignment};
use namumark_ir::{
    Color, ColorValue, Dimension, ImageAlignment, ImageTheme, RenderBackend, RenderBlock,
    RenderInline, RenderTable, RenderTableRow, RenderTree, RenderedFootnote, TableOfContentsEntry,
    TextStyle, VideoProvider,
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

/// 문서 전체. 헤딩 콘텐츠 래퍼(`wiki-heading-content`)의 개폐는
/// 형제 블록 순서에 걸친 상태이므로 문서 래퍼가 소유한다.
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
                    writeln!(formatter, "</div>")?;
                }
                write!(formatter, "{}", BlockMarkup(block))?;
                write!(formatter, "<div class=\"wiki-heading-content\">")?;
                open_heading_levels.push(*level);
            } else {
                write!(formatter, "{}", BlockMarkup(block))?;
            }
        }
        for _ in open_heading_levels.drain(..) {
            writeln!(formatter, "</div>")?;
        }
        if !self.0.categories.is_empty() {
            write!(formatter, "{}", CategoriesMarkup(&self.0.categories))?;
        }
        Ok(())
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
                let tag_level = (*level).clamp(1, 6);
                let folded_class = if *folded { " wiki-heading-folded" } else { "" };
                writeln!(
                    formatter,
                    "<h{tag_level} class=\"wiki-heading{folded_class}\"><a id=\"s-{number}\" href=\"#toc\">{number}.</a> {content}</h{tag_level}>",
                    content = InlinesMarkup(content)
                )
            }
            RenderBlock::Paragraph(inlines) => {
                writeln!(
                    formatter,
                    "<div class=\"wiki-paragraph\">{}</div>",
                    InlinesMarkup(inlines)
                )
            }
            RenderBlock::HorizontalRule => writeln!(formatter, "<hr class=\"wiki-hr\">"),
            RenderBlock::Quote(blocks) => {
                writeln!(
                    formatter,
                    "<blockquote class=\"wiki-quote\">{}</blockquote>",
                    BlocksMarkup(blocks)
                )
            }
            RenderBlock::List { kind, items } => {
                let (tag, list_type) = match kind {
                    ListKind::Unordered => ("ul", None),
                    ListKind::Decimal => ("ol", None),
                    ListKind::LowerAlphabet => ("ol", Some("a")),
                    ListKind::UpperAlphabet => ("ol", Some("A")),
                    ListKind::LowerRoman => ("ol", Some("i")),
                    ListKind::UpperRoman => ("ol", Some("I")),
                };
                write!(formatter, "<{tag} class=\"wiki-list\"")?;
                if let Some(list_type) = list_type {
                    write!(formatter, " type=\"{list_type}\"")?;
                }
                write!(formatter, ">")?;
                for item in items {
                    match item.start_number {
                        Some(start_number) => write!(formatter, "<li value=\"{start_number}\">")?,
                        None => write!(formatter, "<li>")?,
                    }
                    write!(formatter, "{}</li>", BlocksMarkup(&item.blocks))?;
                }
                writeln!(formatter, "</{tag}>")
            }
            RenderBlock::Indent(blocks) => {
                writeln!(
                    formatter,
                    "<div class=\"wiki-indent\">{}</div>",
                    BlocksMarkup(blocks)
                )
            }
            RenderBlock::Table(table) => write!(formatter, "{}", TableMarkup(table)),
            RenderBlock::CodeBlock { language, source } => {
                match language {
                    Some(language) => write!(
                        formatter,
                        "<pre class=\"wiki-code\"><code class=\"language-{}\">",
                        escape_attribute(language)
                    )?,
                    None => write!(formatter, "<pre class=\"wiki-code\"><code>")?,
                }
                writeln!(formatter, "{}</code></pre>", escape_text(source))
            }
            RenderBlock::WikiStyle {
                style,
                dark_style,
                blocks,
            } => {
                write!(
                    formatter,
                    "<div class=\"wiki-style\" style=\"{}\"",
                    escape_attribute(style.as_deref().unwrap_or(""))
                )?;
                if let Some(dark_style) = dark_style {
                    write!(
                        formatter,
                        " data-dark-style=\"{}\"",
                        escape_attribute(dark_style)
                    )?;
                }
                writeln!(formatter, ">{}</div>", BlocksMarkup(blocks))
            }
            RenderBlock::Folding { summary, blocks } => {
                write!(formatter, "<dl class=\"wiki-folding\"><dt>")?;
                if summary.is_empty() {
                    formatter.write_str("More")?;
                } else {
                    write!(formatter, "{}", InlinesMarkup(summary))?;
                }
                writeln!(formatter, "</dt><dd>{}</dd></dl>", BlocksMarkup(blocks))
            }
            RenderBlock::Colored { color, blocks } => {
                writeln!(
                    formatter,
                    "<div class=\"wiki-colored\" style=\"{}\">{}</div>",
                    ColorVariables(color),
                    BlocksMarkup(blocks)
                )
            }
            RenderBlock::Sized { level, blocks } => {
                writeln!(
                    formatter,
                    "<div class=\"{}\">{}</div>",
                    SizeClass(*level),
                    BlocksMarkup(blocks)
                )
            }
            RenderBlock::Html(html) => {
                // sanitizer 도입 전까지 원문을 이스케이프해 표시한다.
                write!(
                    formatter,
                    "<pre class=\"wiki-raw-html\"><code>{}</code></pre>",
                    escape_text(html)
                )
            }
            RenderBlock::TableOfContents { entries } => {
                write!(formatter, "{}", TableOfContentsMarkup(entries))
            }
            RenderBlock::FootnoteSection { notes } => {
                write!(formatter, "{}", FootnoteSectionMarkup(notes))
            }
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
        write!(
            formatter,
            "<div class=\"wiki-macro-toc\" id=\"toc\"><div class=\"toc-head\">목차</div>"
        )?;
        for entry in self.0 {
            write!(
                formatter,
                "<div class=\"toc-item toc-indent-{depth}\"><a href=\"#s-{number}\">{number}.</a> {title}</div>",
                depth = entry.depth,
                number = entry.number,
                title = InlinesMarkup(&entry.title)
            )?;
        }
        writeln!(formatter, "</div>")
    }
}

struct FootnoteSectionMarkup<'notes>(&'notes [RenderedFootnote]);

impl Display for FootnoteSectionMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        write!(formatter, "<div class=\"wiki-macro-footnote\">")?;
        for footnote in self.0 {
            write!(formatter, "{}", FootnoteMarkup(footnote))?;
        }
        writeln!(formatter, "</div>")
    }
}

struct FootnoteMarkup<'footnote>(&'footnote RenderedFootnote);

impl Display for FootnoteMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let footnote = self.0;
        write!(formatter, "<span class=\"footnote-list\">")?;
        if footnote.reference_count <= 1 {
            write!(
                formatter,
                "<a id=\"fn-{label}\" href=\"#rfn-{label}-0\">[{text}]</a> ",
                label = escape_attribute(&footnote.label),
                text = escape_text(&footnote.label)
            )?;
        } else {
            write!(
                formatter,
                "<span id=\"fn-{}\">[{}]</span>",
                escape_attribute(&footnote.label),
                escape_text(&footnote.label)
            )?;
            for reference_index in 0..footnote.reference_count {
                write!(
                    formatter,
                    " <a href=\"#rfn-{}-{reference_index}\">{}.{}</a>",
                    escape_attribute(&footnote.label),
                    escape_text(&footnote.label),
                    reference_index + 1
                )?;
            }
            formatter.write_char(' ')?;
        }
        write!(formatter, "{}</span>", InlinesMarkup(&footnote.content))
    }
}

struct TableMarkup<'table>(&'table RenderTable);

impl Display for TableMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "<div class=\"wiki-table-wrap\"><table class=\"wiki-table\"><tbody>"
        )?;
        if let Some(caption) = &self.0.caption {
            write!(formatter, "<caption>{}</caption>", InlinesMarkup(caption))?;
        }
        for row in &self.0.rows {
            write!(formatter, "{}", TableRowMarkup(row))?;
        }
        writeln!(formatter, "</tbody></table></div>")
    }
}

struct TableRowMarkup<'row>(&'row RenderTableRow);

impl Display for TableRowMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut row_style = String::new();
        for cell in &self.0.cells {
            for attribute in &cell.attributes {
                if attribute.scope == TableAttributeScope::Row {
                    append_table_style(&mut row_style, &attribute.name, &attribute.value);
                }
            }
        }
        if row_style.is_empty() {
            write!(formatter, "<tr>")?;
        } else {
            write!(formatter, "<tr style=\"{}\">", escape_attribute(&row_style))?;
        }
        for cell in &self.0.cells {
            write!(formatter, "{}", TableCellMarkup(cell))?;
        }
        write!(formatter, "</tr>")
    }
}

struct TableCellMarkup<'cell>(&'cell namumark_ir::RenderTableCell);

impl Display for TableCellMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let cell = self.0;
        let mut style = String::new();
        let _ = write!(
            style,
            "text-align: {};",
            match cell.horizontal_alignment {
                HorizontalAlignment::Left => "left",
                HorizontalAlignment::Center => "center",
                HorizontalAlignment::Right => "right",
            }
        );
        if let Some(vertical_alignment) = cell.vertical_alignment {
            let _ = write!(
                style,
                " vertical-align: {};",
                match vertical_alignment {
                    VerticalAlignment::Top => "top",
                    VerticalAlignment::Bottom => "bottom",
                }
            );
        }
        for attribute in &cell.attributes {
            if matches!(
                attribute.scope,
                TableAttributeScope::Cell | TableAttributeScope::Column
            ) {
                append_table_style(&mut style, &attribute.name, &attribute.value);
            }
        }
        write!(formatter, "<td")?;
        if cell.column_span > 1 {
            write!(formatter, " colspan=\"{}\"", cell.column_span)?;
        }
        if cell.row_span > 1 {
            write!(formatter, " rowspan=\"{}\"", cell.row_span)?;
        }
        write!(
            formatter,
            " style=\"{}\">{}</td>",
            escape_attribute(&style),
            BlocksMarkup(&cell.blocks)
        )
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
            RenderInline::LineBreak => formatter.write_str("<br>"),
            RenderInline::Styled { style, content } => {
                let tag = match style {
                    TextStyle::Bold => "strong",
                    TextStyle::Italic => "em",
                    TextStyle::Strikethrough => "del",
                    TextStyle::Underline => "u",
                    TextStyle::Superscript => "sup",
                    TextStyle::Subscript => "sub",
                };
                write!(formatter, "<{tag}>{}</{tag}>", InlinesMarkup(content))
            }
            RenderInline::Literal(text) => {
                write!(formatter, "<code>{}</code>", escape_text(text))
            }
            RenderInline::Colored { color, content } => {
                write!(
                    formatter,
                    "<span class=\"wiki-colored\" style=\"{}\">{}</span>",
                    ColorVariables(color),
                    InlinesMarkup(content)
                )
            }
            RenderInline::Sized { level, content } => {
                write!(
                    formatter,
                    "<span class=\"{}\">{}</span>",
                    SizeClass(*level),
                    InlinesMarkup(content)
                )
            }
            RenderInline::DocumentLink {
                title,
                anchor,
                exists,
                display,
            } => {
                let exists_class = if *exists { "" } else { " not-exist" };
                write!(
                    formatter,
                    "<a class=\"wiki-link-internal{exists_class}\" href=\""
                )?;
                if !title.is_empty() {
                    write!(formatter, "/w/{}", percent_encode(title))?;
                }
                if let Some(anchor) = anchor {
                    write!(formatter, "#{}", percent_encode(anchor))?;
                }
                write!(formatter, "\" title=\"{}\">", escape_attribute(title))?;
                match display {
                    Some(display) => write!(formatter, "{}", InlinesMarkup(display))?,
                    None => write!(formatter, "{}", escape_text(title))?,
                }
                write!(formatter, "</a>")
            }
            RenderInline::ExternalLink { url, display } => {
                let safe_url = if url
                    .trim_start()
                    .to_ascii_lowercase()
                    .starts_with("javascript:")
                {
                    "#"
                } else {
                    url.as_str()
                };
                write!(
                    formatter,
                    "<a class=\"wiki-link-external\" href=\"{}\" target=\"_blank\" rel=\"noopener nofollow\">",
                    escape_attribute(safe_url)
                )?;
                match display {
                    Some(display) => write!(formatter, "{}", InlinesMarkup(display))?,
                    None => write!(formatter, "{}", escape_text(url))?,
                }
                write!(formatter, "</a>")
            }
            RenderInline::Image {
                file_name,
                url,
                layout,
            } => match url {
                Some(url) => {
                    write!(formatter, "<span class=\"wiki-image")?;
                    if let Some(align) = layout.align {
                        let align = match align {
                            ImageAlignment::Left => "left",
                            ImageAlignment::Center => "center",
                            ImageAlignment::Right => "right",
                        };
                        write!(formatter, " wiki-image-align-{align}")?;
                    }
                    if let Some(theme) = layout.theme {
                        let theme = match theme {
                            ImageTheme::Light => "light",
                            ImageTheme::Dark => "dark",
                        };
                        write!(formatter, " wiki-image-theme-{theme}")?;
                    }
                    write!(
                        formatter,
                        "\"><img src=\"{}\" alt=\"{}\" style=\"{}\"></span>",
                        escape_attribute(url),
                        escape_attribute(file_name),
                        ImageStyle(layout)
                    )
                }
                None => {
                    write!(
                        formatter,
                        "<a class=\"wiki-link-internal not-exist\" title=\"파일:{}\">파일:{}</a>",
                        escape_attribute(file_name),
                        escape_text(file_name)
                    )
                }
            },
            RenderInline::FootnoteReference {
                label,
                reference_index,
            } => {
                write!(
                    formatter,
                    "<a class=\"wiki-fn-content\" id=\"rfn-{label_attribute}-{reference_index}\" href=\"#fn-{label_attribute}\">[{label_text}]</a>",
                    label_attribute = escape_attribute(label),
                    label_text = escape_text(label)
                )
            }
            RenderInline::Video {
                provider,
                identifier,
                width,
                height,
            } => {
                write!(formatter, "<iframe class=\"wiki-video\" src=\"")?;
                match provider {
                    VideoProvider::Youtube => write!(
                        formatter,
                        "https://www.youtube.com/embed/{}",
                        percent_encode(identifier)
                    )?,
                    VideoProvider::KakaoTv => write!(
                        formatter,
                        "https://tv.kakao.com/embed/player/cliplink/{}",
                        percent_encode(identifier)
                    )?,
                    VideoProvider::NicoVideo => write!(
                        formatter,
                        "https://embed.nicovideo.jp/watch/{}",
                        percent_encode(identifier)
                    )?,
                }
                write!(
                    formatter,
                    "\" width=\"{}\" height=\"{}\" frameborder=\"0\" allowfullscreen loading=\"lazy\"></iframe>",
                    escape_attribute(width.as_deref().unwrap_or("640")),
                    escape_attribute(height.as_deref().unwrap_or("360"))
                )
            }
            RenderInline::Ruby { content, ruby } => {
                write!(
                    formatter,
                    "<ruby>{}<rp>(</rp><rt>{}</rt><rp>)</rp></ruby>",
                    escape_text(content),
                    escape_text(ruby)
                )
            }
            RenderInline::Math { formula } => {
                write!(
                    formatter,
                    "<span class=\"wiki-math\" data-formula=\"{}\">\\({}\\)</span>",
                    escape_attribute(formula),
                    escape_text(formula)
                )
            }
            RenderInline::Anchor { name } => {
                write!(formatter, "<a id=\"{}\"></a>", escape_attribute(name))
            }
            RenderInline::ClearFix => formatter.write_str("<div class=\"wiki-clearfix\"></div>"),
            RenderInline::Html(html) => {
                write!(formatter, "<code>{}</code>", escape_text(html))
            }
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

struct CategoriesMarkup<'categories>(&'categories [String]);

impl Display for CategoriesMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "<div class=\"wiki-categories\">분류:")?;
        for (index, category) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str(" | ")?;
            }
            write!(
                formatter,
                " <a href=\"/w/%EB%B6%84%EB%A5%98%3A{}\">{}</a>",
                percent_encode(category),
                escape_text(category)
            )?;
        }
        writeln!(formatter, "</div>")
    }
}

struct ColorVariables<'color>(&'color Color);

impl Display for ColorVariables<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let dark = self.0.dark.as_ref().unwrap_or(&self.0.light);
        write!(
            formatter,
            "--wiki-color: {}; --wiki-color-dark: {};",
            escape_attribute(&self.0.light),
            escape_attribute(dark)
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

struct ImageStyle<'layout>(&'layout namumark_ir::ImageLayout);

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

fn append_table_style(style: &mut String, name: &str, value: &Option<String>) {
    let Some(value) = value else { return };
    // 듀얼 색상(`#fff,#000`)은 라이트 값을 쓴다. 다크 모드 값은 후속 과제.
    let value = value.split(',').next().unwrap_or(value);
    let _ = match name {
        "bgcolor" => write!(style, " background-color: {};", ColorValue::parse(value)),
        "color" => write!(style, " color: {};", ColorValue::parse(value)),
        "width" => write!(style, " width: {};", Dimension::parse(value)),
        "height" => write!(style, " height: {};", Dimension::parse(value)),
        "textalign" => write!(style, " text-align: {};", value.trim()),
        _ => Ok(()),
    };
}

// ---- 이스케이프 어댑터 (중간 문자열 없이 Formatter로 직접 방출) ----

fn escape_text<T: Display>(value: T) -> impl Display {
    EscapeAdapter {
        value,
        escape_quotes: false,
    }
}

fn escape_attribute<T: Display>(value: T) -> impl Display {
    EscapeAdapter {
        value,
        escape_quotes: true,
    }
}

struct EscapeAdapter<T> {
    value: T,
    escape_quotes: bool,
}

impl<T: Display> Display for EscapeAdapter<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut escaper = EscapingWriter {
            inner: formatter,
            escape_quotes: self.escape_quotes,
        };
        write!(escaper, "{}", self.value)
    }
}

struct EscapingWriter<'writer, 'buffer> {
    inner: &'writer mut Formatter<'buffer>,
    escape_quotes: bool,
}

impl fmt::Write for EscapingWriter<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            match character {
                '&' => self.inner.write_str("&amp;")?,
                '<' => self.inner.write_str("&lt;")?,
                '>' => self.inner.write_str("&gt;")?,
                '"' if self.escape_quotes => self.inner.write_str("&quot;")?,
                '\'' if self.escape_quotes => self.inner.write_str("&#x27;")?,
                _ => self.inner.write_char(character)?,
            }
        }
        Ok(())
    }
}

fn percent_encode(text: &str) -> impl Display + '_ {
    PercentEncode(text)
}

struct PercentEncode<'text>(&'text str);

impl Display for PercentEncode<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    formatter.write_char(byte as char)?
                }
                _ => write!(formatter, "%{byte:02X}")?,
            }
        }
        Ok(())
    }
}

/// 나무위키 동등 마크업용 동봉 스타일시트.
pub fn stylesheet() -> &'static str {
    include_str!("../assets/namumark.css")
}
