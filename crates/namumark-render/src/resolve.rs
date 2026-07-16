//! resolve pass: Document → 특화 IR.
//!
//! 외부 세계(WikiContext)를 보는 유일한 pass다. 매크로를 의미 노드로 특화하고,
//! 링크 존재 여부를 해석하고, include를 확장하고, 분류를 수집한다.
//! 블록 성격의 매크로([목차], [각주], [include])는 문단을 분할해 블록으로 승격한다.

use crate::context::WikiContext;
use namumark_ast::{Block, Document, Inline};
use namumark_ir::{
    Color, ColorValue, Dimension, ImageAlignment, ImageLayout, ImageTheme, RenderBlock,
    RenderInline, RenderListItem, RenderTable, RenderTableCell, RenderTableRow, TextStyle,
    VideoProvider,
};

pub(crate) struct Resolved {
    pub redirect: Option<String>,
    pub blocks: Vec<RenderBlock>,
    pub categories: Vec<String>,
}

pub(crate) fn resolve(document: &Document, context: &dyn WikiContext) -> Resolved {
    let mut resolver = Resolver {
        context,
        categories: Vec::new(),
        redirect: None,
        include_stack: Vec::new(),
    };
    let blocks = resolver.resolve_blocks(&document.blocks);
    Resolved {
        redirect: resolver.redirect,
        blocks,
        categories: resolver.categories,
    }
}

const MAXIMUM_INCLUDE_DEPTH: usize = 8;

struct Resolver<'context> {
    context: &'context dyn WikiContext,
    categories: Vec<String>,
    redirect: Option<String>,
    include_stack: Vec<String>,
}

impl Resolver<'_> {
    fn resolve_blocks(&mut self, blocks: &[Block]) -> Vec<RenderBlock> {
        let mut resolved = Vec::new();
        for block in blocks {
            match block {
                Block::Heading(heading) => resolved.push(RenderBlock::Heading {
                    level: heading.level,
                    folded: heading.folded,
                    number: String::new(),
                    content: self.resolve_inlines(&heading.content),
                }),
                Block::Paragraph(inlines) => {
                    resolved.extend(self.resolve_paragraph(inlines));
                }
                Block::HorizontalRule => resolved.push(RenderBlock::HorizontalRule),
                Block::Quote(blocks) => {
                    resolved.push(RenderBlock::Quote(self.resolve_blocks(blocks)));
                }
                Block::List(list) => resolved.push(RenderBlock::List {
                    kind: list.kind,
                    items: list
                        .items
                        .iter()
                        .map(|item| RenderListItem {
                            start_number: item.start_number,
                            blocks: self.resolve_blocks(&item.blocks),
                        })
                        .collect(),
                }),
                Block::Indent(blocks) => {
                    resolved.push(RenderBlock::Indent(self.resolve_blocks(blocks)));
                }
                Block::Table(table) => resolved.push(RenderBlock::Table(RenderTable {
                    caption: table
                        .caption
                        .as_ref()
                        .map(|caption| self.resolve_inlines(caption)),
                    rows: table
                        .rows
                        .iter()
                        .map(|row| RenderTableRow {
                            cells: row
                                .cells
                                .iter()
                                .map(|cell| RenderTableCell {
                                    column_span: cell.column_span,
                                    row_span: cell.row_span,
                                    horizontal_alignment: cell.horizontal_alignment,
                                    vertical_alignment: cell.vertical_alignment,
                                    attributes: cell.attributes.clone(),
                                    blocks: self.resolve_blocks(&cell.blocks),
                                })
                                .collect(),
                        })
                        .collect(),
                })),
                Block::CodeBlock(code_block) => resolved.push(RenderBlock::CodeBlock {
                    language: code_block.language.clone(),
                    source: code_block.source.clone(),
                }),
                Block::WikiStyle(wiki_style) => resolved.push(RenderBlock::WikiStyle {
                    style: wiki_style.style.clone(),
                    dark_style: wiki_style.dark_style.clone(),
                    blocks: self.resolve_blocks(&wiki_style.blocks),
                }),
                Block::Folding(folding) => resolved.push(RenderBlock::Folding {
                    summary: self.resolve_inlines(&folding.summary),
                    blocks: self.resolve_blocks(&folding.blocks),
                }),
                Block::Colored(colored) => resolved.push(RenderBlock::Colored {
                    color: Color {
                        light: ColorValue::parse(&colored.color),
                        dark: colored.dark_color.as_deref().map(ColorValue::parse),
                    },
                    blocks: self.resolve_blocks(&colored.blocks),
                }),
                Block::Sized(sized) => resolved.push(RenderBlock::Sized {
                    level: sized.level,
                    blocks: self.resolve_blocks(&sized.blocks),
                }),
                Block::Html(html) => resolved.push(RenderBlock::Html(html.clone())),
                Block::Comment(_) => {}
                Block::Redirect(target) => {
                    if self.redirect.is_none() && self.include_stack.is_empty() {
                        self.redirect = Some(target.clone());
                    }
                }
            }
        }
        resolved
    }

    // 문단 안의 블록 성격 매크로([목차]·[각주]·[include])를 만나면 문단을 분할한다.
    fn resolve_paragraph(&mut self, inlines: &[Inline]) -> Vec<RenderBlock> {
        let mut blocks = Vec::new();
        let mut current: Vec<RenderInline> = Vec::new();
        let flush = |current: &mut Vec<RenderInline>, blocks: &mut Vec<RenderBlock>| {
            while current.last() == Some(&RenderInline::LineBreak) {
                current.pop();
            }
            while current.first() == Some(&RenderInline::LineBreak) {
                current.remove(0);
            }
            if !current.is_empty() {
                blocks.push(RenderBlock::Paragraph(std::mem::take(current)));
            } else {
                current.clear();
            }
        };

        for inline in inlines {
            if let Inline::Macro(macro_call) = inline {
                let name = macro_call.name.to_ascii_lowercase();
                match name.as_str() {
                    "목차" | "tableofcontents" => {
                        flush(&mut current, &mut blocks);
                        blocks.push(RenderBlock::TableOfContents {
                            entries: Vec::new(),
                        });
                        continue;
                    }
                    "각주" | "footnote" => {
                        flush(&mut current, &mut blocks);
                        blocks.push(RenderBlock::FootnoteSection { notes: Vec::new() });
                        continue;
                    }
                    "include" => {
                        flush(&mut current, &mut blocks);
                        blocks.extend(self.expand_include(macro_call.argument.as_deref()));
                        continue;
                    }
                    _ => {}
                }
            }
            if let Some(resolved) = self.resolve_inline(inline) {
                current.push(resolved);
            }
        }
        flush(&mut current, &mut blocks);
        blocks
    }

    fn resolve_inlines(&mut self, inlines: &[Inline]) -> Vec<RenderInline> {
        inlines
            .iter()
            .filter_map(|inline| self.resolve_inline(inline))
            .collect()
    }

    fn resolve_inline(&mut self, inline: &Inline) -> Option<RenderInline> {
        Some(match inline {
            Inline::Text(text) => RenderInline::Text(text.clone()),
            Inline::LineBreak => RenderInline::LineBreak,
            Inline::Bold(content) => self.resolve_styled(TextStyle::Bold, content),
            Inline::Italic(content) => self.resolve_styled(TextStyle::Italic, content),
            Inline::Strikethrough(content) => {
                self.resolve_styled(TextStyle::Strikethrough, content)
            }
            Inline::Underline(content) => self.resolve_styled(TextStyle::Underline, content),
            Inline::Superscript(content) => self.resolve_styled(TextStyle::Superscript, content),
            Inline::Subscript(content) => self.resolve_styled(TextStyle::Subscript, content),
            Inline::Literal(text) => RenderInline::Literal(text.clone()),
            Inline::Colored(colored) => RenderInline::Colored {
                color: Color {
                    light: ColorValue::parse(&colored.color),
                    dark: colored.dark_color.as_deref().map(ColorValue::parse),
                },
                content: self.resolve_inlines(&colored.content),
            },
            Inline::Sized(sized) => RenderInline::Sized {
                level: sized.level,
                content: self.resolve_inlines(&sized.content),
            },
            Inline::Link(link) => {
                let display = link
                    .display
                    .as_ref()
                    .map(|display| self.resolve_inlines(display));
                if is_external_url(&link.target) {
                    RenderInline::ExternalLink {
                        url: link.target.clone(),
                        display,
                    }
                } else {
                    RenderInline::DocumentLink {
                        title: link.target.clone(),
                        anchor: link.anchor.clone(),
                        exists: link.target.is_empty()
                            || self.context.document_exists(&link.target),
                        display,
                    }
                }
            }
            Inline::Image(image) => {
                let mut layout = ImageLayout::default();
                for option in &image.options {
                    let Some(value) = option.value.as_deref() else {
                        continue;
                    };
                    match option.name.as_str() {
                        "width" => layout.width = Some(Dimension::parse(value)),
                        "height" => layout.height = Some(Dimension::parse(value)),
                        "align" => {
                            layout.align = match value.trim() {
                                "left" => Some(ImageAlignment::Left),
                                "center" => Some(ImageAlignment::Center),
                                "right" => Some(ImageAlignment::Right),
                                _ => None,
                            }
                        }
                        "bgcolor" => layout.background_color = Some(ColorValue::parse(value)),
                        "theme" => {
                            layout.theme = match value.trim() {
                                "light" => Some(ImageTheme::Light),
                                "dark" => Some(ImageTheme::Dark),
                                _ => None,
                            }
                        }
                        _ => {}
                    }
                }
                RenderInline::Image {
                    url: self.context.file_url(&image.file_name),
                    file_name: image.file_name.clone(),
                    layout,
                }
            }
            Inline::Category(category) => {
                if !self.categories.contains(&category.name) {
                    self.categories.push(category.name.clone());
                }
                return None;
            }
            Inline::Footnote(footnote) => RenderInline::Footnote {
                name: footnote.name.clone(),
                content: self.resolve_inlines(&footnote.content),
            },
            Inline::Macro(macro_call) => {
                self.resolve_macro(&macro_call.name, macro_call.argument.as_deref())
            }
            Inline::Html(html) => RenderInline::Html(html.clone()),
        })
    }

    fn resolve_styled(&mut self, style: TextStyle, content: &[Inline]) -> RenderInline {
        RenderInline::Styled {
            style,
            content: self.resolve_inlines(content),
        }
    }

    fn resolve_macro(&mut self, name: &str, argument: Option<&str>) -> RenderInline {
        let unresolved = || RenderInline::Unresolved {
            name: name.to_string(),
            argument: argument.map(str::to_string),
        };
        match name.to_ascii_lowercase().as_str() {
            "br" => RenderInline::LineBreak,
            "clearfix" => RenderInline::ClearFix,
            "anchor" => match argument {
                Some(anchor_name) => RenderInline::Anchor {
                    name: anchor_name.to_string(),
                },
                None => unresolved(),
            },
            "math" => match argument {
                Some(formula) => RenderInline::Math {
                    formula: formula.to_string(),
                },
                None => unresolved(),
            },
            "date" | "datetime" => match self.context.now() {
                Some(now) => RenderInline::Text(now.to_string()),
                None => unresolved(),
            },
            "age" => match (argument.and_then(parse_date), self.context.now()) {
                (Some(birth), Some(now)) => {
                    let today = now.date;
                    let mut age = today.year - birth.year;
                    if (today.month, today.day) < (birth.month, birth.day) {
                        age -= 1;
                    }
                    RenderInline::Text(age.to_string())
                }
                _ => unresolved(),
            },
            "dday" => match (argument.and_then(parse_date), self.context.now()) {
                (Some(target), Some(now)) => {
                    let difference = now.date.julian_day_number() - target.julian_day_number();
                    let text = match difference {
                        0 => "D-Day".to_string(),
                        positive if positive > 0 => format!("D+{positive}"),
                        negative => format!("D{negative}"),
                    };
                    RenderInline::Text(text)
                }
                _ => unresolved(),
            },
            "youtube" => self.resolve_video(VideoProvider::Youtube, argument, unresolved),
            "kakaotv" => self.resolve_video(VideoProvider::KakaoTv, argument, unresolved),
            "nicovideo" => self.resolve_video(VideoProvider::NicoVideo, argument, unresolved),
            "ruby" => match argument.and_then(parse_ruby) {
                Some((content, ruby)) => RenderInline::Ruby { content, ruby },
                None => unresolved(),
            },
            _ => unresolved(),
        }
    }

    fn resolve_video(
        &mut self,
        provider: VideoProvider,
        argument: Option<&str>,
        unresolved: impl Fn() -> RenderInline,
    ) -> RenderInline {
        let Some(argument) = argument else {
            return unresolved();
        };
        let mut parts = argument.split(',');
        let identifier = parts.next().unwrap_or_default().trim().to_string();
        if identifier.is_empty() {
            return unresolved();
        }
        let mut width = None;
        let mut height = None;
        for part in parts {
            if let Some((key, value)) = part.split_once('=') {
                match key.trim() {
                    "width" => width = Some(value.trim().to_string()),
                    "height" => height = Some(value.trim().to_string()),
                    _ => {}
                }
            }
        }
        RenderInline::Video {
            provider,
            identifier,
            width,
            height,
        }
    }

    fn expand_include(&mut self, argument: Option<&str>) -> Vec<RenderBlock> {
        let unresolved = |argument: Option<&str>| {
            vec![RenderBlock::Paragraph(vec![RenderInline::Unresolved {
                name: "include".to_string(),
                argument: argument.map(str::to_string),
            }])]
        };
        let Some(argument) = argument else {
            return unresolved(argument);
        };
        let mut parts = argument.split(',');
        let title = parts.next().unwrap_or_default().trim().to_string();
        if title.is_empty()
            || self.include_stack.len() >= MAXIMUM_INCLUDE_DEPTH
            || self.include_stack.contains(&title)
        {
            return unresolved(Some(argument));
        }
        let Some(source) = self.context.include_source(&title) else {
            return unresolved(Some(argument));
        };
        // 인자 치환: 틀 본문의 `@이름@`을 호출 인자로 바꾼다.
        let mut substituted = source;
        for part in parts {
            if let Some((key, value)) = part.split_once('=') {
                substituted = substituted.replace(&format!("@{}@", key.trim()), value.trim());
            }
        }
        let document = namumark_parser::parse(&substituted);
        self.include_stack.push(title);
        let blocks = self.resolve_blocks(&document.blocks);
        self.include_stack.pop();
        blocks
    }
}

fn is_external_url(target: &str) -> bool {
    ["http://", "https://", "ftp://"].iter().any(|scheme| {
        target.len() >= scheme.len()
            && target.as_bytes()[..scheme.len()].eq_ignore_ascii_case(scheme.as_bytes())
    })
}

fn parse_date(source: &str) -> Option<crate::context::Date> {
    let mut parts = source.trim().split('-');
    let year = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(crate::context::Date { year, month, day })
}

fn parse_ruby(argument: &str) -> Option<(String, String)> {
    let (content, options) = argument.split_once(',')?;
    let ruby = options
        .split(',')
        .find_map(|part| part.trim().strip_prefix("ruby="))?;
    Some((content.trim().to_string(), ruby.trim().to_string()))
}
