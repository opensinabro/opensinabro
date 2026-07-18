//! resolve pass: Document → 특화 IR.
//!
//! 외부 세계(WikiContext)를 보는 유일한 pass다. 매크로를 의미 노드로 특화하고,
//! 링크 존재 여부를 해석하고, include를 확장하고, 분류를 수집한다.
//! 블록 성격의 매크로([목차], [각주], [include])는 문단을 분할해 블록으로 승격한다.

use crate::condition;
use crate::context::WikiContext;
use namumark_analysis::{Diagnostic, DiagnosticCode, TextRange};
use namumark_ast::{
    AstNode, Block, Conditional, Document, Fragment, HorizontalAlignment, Inline, Template,
};
use namumark_ir::{
    Color, ColorValue, Dimension, DocumentLinkKind, ImageAlignment, ImageLayout, ImageTheme,
    RenderBlock, RenderInline, RenderListItem, RenderTable, RenderTableAttribute, RenderTableCell,
    RenderTableRow, StyleDeclaration, TableStyleProperty, TextStyle, VideoProvider,
};
use std::collections::HashMap;

/// 문서 안 include 인스턴스의 순번(1부터). 나무위키는 틀 안에서 쓴 같은 문서 앵커에
/// 이 번호를 접두사로 붙여 틀끼리 앵커가 겹치지 않게 한다.
#[derive(Debug, Clone, Copy)]
struct IncludeInstance(u32);

impl IncludeInstance {
    /// 렌더확정: 3번째 include인 `틀:다른 뜻` 안의 `[[#s-@paragraph1@]]`가
    /// the seed에서 `href='#i3-s-'`로 나온다.
    fn qualify(self, anchor: &str) -> String {
        format!("i{}-{anchor}", self.0)
    }
}

pub(crate) struct Resolved {
    pub redirect: Option<String>,
    pub blocks: Vec<RenderBlock>,
    pub categories: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn resolve(document: &Document, context: &dyn WikiContext) -> Resolved {
    let mut resolver = Resolver {
        context,
        categories: Vec::new(),
        redirect: None,
        include_instance: None,
        expanded_includes: 0,
        scope: HashMap::new(),
        in_cell: false,
        diagnostics: Vec::new(),
    };
    let blocks = resolver.resolve_blocks(&document.blocks());
    Resolved {
        redirect: resolver.redirect,
        blocks,
        categories: resolver.categories,
        diagnostics: resolver.diagnostics,
    }
}

struct Resolver<'context> {
    context: &'context dyn WikiContext,
    categories: Vec<String>,
    redirect: Option<String>,
    /// 지금 확장 중인 include가 문서의 몇 번째인가. 나무위키는 틀 속의 틀을 확장하지
    /// 않으므로 깊이는 최대 1이고, 이 값 하나로 "안에 있는가"까지 같이 나타낸다.
    include_instance: Option<IncludeInstance>,
    /// 지금까지 확장한 include 수. 다음 인스턴스의 번호가 된다.
    expanded_includes: u32,
    /// 틀 인자와 `#!if`가 만든 변수. `@이름@`의 값이 여기서 정해진다.
    scope: HashMap<String, String>,
    /// 지금 표 셀 안을 해석 중인가. 셀 안에서는 뒤가 invisible(분류·include)이어도
    /// 콘텐츠 뒤 개행이 `<br>`로 남는다(렌더확정: 표 셀 안 `[include(틀:글무리)]`).
    in_cell: bool,
    /// resolve 중 모인 진단(문맥 의존). 편집 중인 문서 자신의 지점만 담도록,
    /// include로 확장한 틀 내부(`include_instance.is_some()`)에서는 방출하지 않는다.
    diagnostics: Vec<Diagnostic>,
}

impl Resolver<'_> {
    /// 틀 인자를 값으로 바꿔 문자열을 완성한다. 인자 > 기본값 > 빈 문자열 순이다.
    fn fill(&self, template: &Template) -> String {
        let mut output = String::new();
        for fragment in &template.0 {
            match fragment {
                Fragment::Text(text) => output.push_str(text),
                Fragment::Variable(variable) => output.push_str(
                    &self
                        .scope
                        .get(&variable.name)
                        .cloned()
                        .or_else(|| variable.default.clone())
                        .unwrap_or_default(),
                ),
            }
        }
        output
    }

    fn fill_option(&self, template: &Option<Template>) -> Option<String> {
        template.as_ref().map(|template| self.fill(template))
    }

    /// `#!wiki`의 style 인자를 채운 뒤 나무위키가 받아들이는 선언만 걸러 남긴다.
    fn fill_style(&self, template: &Option<Template>) -> Vec<StyleDeclaration> {
        match template {
            Some(template) => StyleDeclaration::parse(&self.fill(template)),
            None => Vec::new(),
        }
    }

    /// 표 속성의 틀 인자를 채워 값으로 확정하고, 방출되는 속성만 타입화해 남긴다.
    fn resolve_table_attribute(
        &self,
        attribute: &namumark_ast::TableAttribute,
    ) -> Option<RenderTableAttribute> {
        let value = self.fill_option(&attribute.value);
        Some(RenderTableAttribute {
            scope: attribute.scope,
            property: table_style_property(&attribute.name, value.as_deref())?,
        })
    }

    fn resolve_blocks(&mut self, blocks: &[Block]) -> Vec<RenderBlock> {
        let mut resolved = Vec::new();
        for block in blocks {
            match block {
                Block::Heading(heading) => resolved.push(RenderBlock::Heading {
                    level: heading.level(),
                    folded: heading.folded(),
                    number: String::new(),
                    anchor: String::new(),
                    content: self.resolve_inlines(&heading.content()),
                }),
                Block::Paragraph(paragraph) => {
                    resolved.extend(self.resolve_paragraph(&paragraph.inlines()));
                }
                Block::HorizontalRule => resolved.push(RenderBlock::HorizontalRule),
                Block::Quote(quote) => {
                    resolved.push(RenderBlock::Quote(self.resolve_blocks(&quote.blocks())));
                }
                Block::List(list) => resolved.push(RenderBlock::List {
                    kind: list.kind(),
                    items: list
                        .items()
                        .iter()
                        .map(|item| RenderListItem {
                            start_number: item.start_number(),
                            blocks: self.resolve_blocks(&item.blocks()),
                        })
                        .collect(),
                }),
                Block::Indent(indent) => {
                    resolved.push(RenderBlock::Indent(self.resolve_blocks(&indent.blocks())));
                }
                Block::Table(table) => resolved.push(RenderBlock::Table(RenderTable {
                    caption: table
                        .caption()
                        .as_ref()
                        .map(|caption| self.resolve_inlines(caption)),
                    rows: table
                        .rows()
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
                                    attributes: cell
                                        .attributes
                                        .iter()
                                        .filter_map(|attribute| {
                                            self.resolve_table_attribute(attribute)
                                        })
                                        .collect(),
                                    blocks: {
                                        let previous = self.in_cell;
                                        self.in_cell = true;
                                        let resolved = self.resolve_blocks(&cell.blocks);
                                        self.in_cell = previous;
                                        resolved
                                    },
                                })
                                .collect(),
                        })
                        .collect(),
                })),
                Block::Comment(_) => {}
                Block::Redirect(redirect) => {
                    if self.redirect.is_none() && self.include_instance.is_none() {
                        let target = self.fill(&redirect.target());
                        if self.context.current_title().as_deref() == Some(target.as_str()) {
                            self.diagnostics.push(Diagnostic {
                                code: DiagnosticCode::SelfRedirect,
                                range: redirect.syntax().text_range(),
                                message: "리다이렉트 대상이 이 문서 자신이라 순환합니다.".into(),
                                suggestion: None,
                            });
                        }
                        self.redirect = Some(target);
                    }
                }
            }
        }
        resolved
    }

    // 문단 안의 블록 성격 매크로([목차]·[각주]·[include])를 만나면 문단을 분할한다.
    fn resolve_paragraph(&mut self, inlines: &[Inline]) -> Vec<RenderBlock> {
        // 원문에 있던 빈 문단은 화면에도 빈 문단으로 남는다(렌더확정: 리스트 항목의
        // 속내용 뒤 빈 줄이 the seed에서 `<div class='wiki-paragraph'></div>`가 된다).
        if inlines.is_empty() {
            return vec![RenderBlock::Paragraph(Vec::new())];
        }
        let mut blocks = Vec::new();
        let mut current: Vec<RenderInline> = Vec::new();
        // 문단 가장자리의 줄바꿈도 화면에 남는다 — 나무위키는 빈 줄을 그대로 보여준다.
        let flush = |current: &mut Vec<RenderInline>, blocks: &mut Vec<RenderBlock>| {
            if !current.is_empty() {
                blocks.push(RenderBlock::Paragraph(std::mem::take(current)));
            } else {
                current.clear();
            }
        };

        // 줄 단위로 본다 — include가 제 문단을 이루는지(줄 첫머리) 같은 문단에 중첩되는지
        // (줄 중간, 각주 뒤 `[각주][include]`)를 줄 내 위치로 가른다.
        let lines: Vec<&[Inline]> = inlines
            .split(|inline| matches!(inline, Inline::LineBreak))
            .collect();
        // 블록으로 승격한 include가 있었는지. 그 뒤 빈 줄은 화면에 빈 문단으로 남는다.
        let mut promoted_include = false;
        for (index, line) in lines.iter().enumerate() {
            let rest_invisible = lines[index + 1..]
                .iter()
                .all(|line| is_invisible_line(line));
            let line_start = current.len();
            for inline in *line {
                if let Inline::Conditional(conditional) = inline {
                    current.extend(self.resolve_conditional(conditional));
                    continue;
                }
                if let Inline::Macro(macro_call) = inline
                    && macro_call.name().eq_ignore_ascii_case("include")
                {
                    let argument = self.fill_option(&macro_call.argument());
                    let expanded =
                        self.expand_include(argument.as_deref(), macro_call.syntax().text_range());
                    // 줄에 앞선 내용이 있으면(각주 뒤 `[각주][include(틀:문서 가져옴)]`) 그 문단에
                    // 블록째 중첩된다(렌더확정: the seed의 바깥 wiki-paragraph 하나가 각주 섹션과
                    // 문서 가져옴을 함께 감싼다). include가 줄 첫머리인데 뒤에 보이는 내용이 있으면
                    // (`[include(틀:국기)][각주]`) 확장 문단을 인라인으로 펴 같은 문단에 이어 붙이고
                    // (렌더확정: the seed는 정보상자 셀에서 국기와 각주를 wrapper 없이 한 문단에
                    // 나란히 둔다), 줄이 분류·include뿐이면 제 문단을 이룬다.
                    if current.len() > line_start {
                        current.push(RenderInline::Blocks(expanded));
                    } else if !is_invisible_line(line) {
                        current.extend(flatten_render_blocks(expanded));
                    } else {
                        flush(&mut current, &mut blocks);
                        blocks.extend(expanded);
                        promoted_include = true;
                    }
                    continue;
                }
                if let Some(resolved) = self.resolve_inline(inline) {
                    current.push(resolved);
                }
            }
            // 블록으로 승격한 include 뒤의 빈 줄은 붙을 데가 없어 빈 문단으로 남는다
            // (렌더확정: `[include(틀:상세 내용)]` 다음 빈 줄과 헤딩 사이에 the seed는
            // `<div class='wiki-paragraph'></div>`를 둔다). 앞에 보이는 내용이 있으면 그 br로
            // 붙으므로(current 비지 않음) 여기 걸리지 않는다.
            if line.is_empty() && promoted_include && current.is_empty() {
                blocks.push(RenderBlock::Paragraph(Vec::new()));
            }
            // 줄 사이 개행은 화면에 br로 남는다. 단 뒤가 전부 invisible(분류·include)이면
            // 그 앞 개행까지 사라진다 — 표 셀 안에서는 남는다(규칙 (3)·셀 문맥).
            if index + 1 < lines.len()
                && !is_invisible_line(line)
                && (self.in_cell || !rest_invisible)
            {
                current.push(RenderInline::LineBreak);
            }
        }
        flush(&mut current, &mut blocks);
        blocks
    }

    fn resolve_inlines(&mut self, inlines: &[Inline]) -> Vec<RenderInline> {
        let mut resolved = Vec::new();
        for inline in inlines {
            if let Inline::Conditional(conditional) = inline {
                resolved.extend(self.resolve_conditional(conditional));
                continue;
            }
            if let Some(inline) = self.resolve_inline(inline) {
                resolved.push(inline);
            }
        }
        resolved
    }

    /// 현재 문서를 기준으로 한 상대 링크를 문서명으로 편다.
    ///
    /// 렌더확정: `알파위키:문법 도움말/심화`에서 `[[../#문법 무효화|…]]`는
    /// `/w/알파위키:문법 도움말#문법 무효화`로, `알파위키:문법 도움말`에서 `[[/심화]]`는
    /// `/w/알파위키:문법 도움말/심화`로 간다. `../`는 여러 번 써도 한 단계만 올라가므로
    /// 첫 하나만 해석한다(상위가 없으면 현재 문서 자신).
    fn resolve_link_target(&self, written: &str) -> String {
        // `문서:`는 이름공간이 아니라 "본문 이름공간"을 못박는 표시다. 제목이 `/`로
        // 시작해 하위 문서로 읽히는 것을 막을 때 쓴다(렌더확정: `[[문서:/// (너 먹구름 비)]]`가
        // the seed에서 `/w////%20(%EB%84%88%20…)`로 간다).
        if let Some(absolute) = written.strip_prefix("문서:") {
            return absolute.to_string();
        }
        let Some(current) = self.context.current_title() else {
            return written.to_string();
        };
        if let Some(rest) = written.strip_prefix("../") {
            let parent = match current.rsplit_once('/') {
                Some((parent, _)) => parent,
                None => &current,
            };
            return if rest.is_empty() {
                parent.to_string()
            } else {
                format!("{parent}/{rest}")
            };
        }
        match written.strip_prefix('/') {
            Some(child) => format!("{current}/{child}"),
            None => written.to_string(),
        }
    }

    /// 렌더확정: `[[#앵커]]`처럼 앵커만 있는 링크는 문서 자신을 가리켜도 자기 링크가
    /// 아니라 `wiki-link-internal`이다(the seed). 문서로 가는 링크만 자기 링크가 된다.
    /// 틀 안에서 쓴 **같은 문서** 앵커에만 인스턴스 번호를 붙인다. 다른 문서의 앵커는
    /// 그 문서의 것이라 건드리지 않는다(렌더확정: `틀:다른 뜻`의 `[[@rd1@#s-@paragraph1@]]`가
    /// rd1이 있으면 `/w/알파위키:기능 도움말#s-5`로 그대로 간다).
    fn qualify_anchor(&self, title: &str, anchor: String) -> String {
        match self.include_instance {
            Some(instance) if title.is_empty() => instance.qualify(&anchor),
            _ => anchor,
        }
    }

    fn link_kind(&self, title: &str, anchor: Option<&str>) -> DocumentLinkKind {
        // 지금 보고 있는 문서는 물어볼 것도 없이 있다.
        if title.is_empty() || self.context.current_title().as_deref() == Some(title) {
            return match anchor {
                Some(_) => DocumentLinkKind::Existing,
                None => DocumentLinkKind::Current,
            };
        }
        if self.context.document_exists(title) {
            DocumentLinkKind::Existing
        } else {
            DocumentLinkKind::Missing
        }
    }

    /// `#!if`는 감싸는 요소를 만들지 않는다 — 조건이 참이면 내용만 남는다.
    fn resolve_conditional(&mut self, conditional: &Conditional) -> Vec<RenderInline> {
        // 조건식은 값을 내면서 변수도 만든다. 만들어진 변수는 뒤따르는
        // `#!if`와 `@이름@`이 함께 쓰므로 스코프에 남긴다.
        let Some(bindings) = condition::evaluate(&conditional.expression(), &self.scope) else {
            return Vec::new();
        };
        self.scope.extend(bindings);
        self.resolve_blocks_as_inlines(&conditional.blocks())
    }

    /// `#!if` 안의 블록들을 감싸는 요소 없이 인라인으로 편다.
    fn resolve_blocks_as_inlines(&mut self, blocks: &[Block]) -> Vec<RenderInline> {
        let mut inlines = Vec::new();
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    if !inlines.is_empty() {
                        inlines.push(RenderInline::LineBreak);
                    }
                    inlines.extend(self.resolve_inlines(&paragraph.inlines()));
                }
                // 표·리스트는 인라인으로 펼 수 없다. 감싸는 요소 없이 담아 둔다.
                other => inlines.push(RenderInline::Blocks(
                    self.resolve_blocks(std::slice::from_ref(other)),
                )),
            }
        }
        inlines
    }

    fn resolve_inline(&mut self, inline: &Inline) -> Option<RenderInline> {
        Some(match inline {
            Inline::Text(text) => RenderInline::Text(text.clone()),
            Inline::LineBreak => RenderInline::LineBreak,
            Inline::Bold(bold) => self.resolve_styled(TextStyle::Bold, &bold.content()),
            Inline::Italic(italic) => self.resolve_styled(TextStyle::Italic, &italic.content()),
            Inline::Strikethrough(struck) => {
                self.resolve_styled(TextStyle::Strikethrough, &struck.content())
            }
            Inline::Underline(underline) => {
                self.resolve_styled(TextStyle::Underline, &underline.content())
            }
            Inline::Superscript(superscript) => {
                self.resolve_styled(TextStyle::Superscript, &superscript.content())
            }
            Inline::Subscript(subscript) => {
                self.resolve_styled(TextStyle::Subscript, &subscript.content())
            }
            Inline::Literal(text) => RenderInline::Literal(text.clone()),
            // 문법이 색상 그룹으로 인정한 표기라 색 판정은 이미 끝나 있다.
            Inline::Colored(colored) => RenderInline::Colored {
                color: Color {
                    light: ColorValue::parse_known(&colored.color()),
                    dark: colored.dark_color().as_deref().map(ColorValue::parse_known),
                },
                content: self.resolve_inlines(&colored.content()),
            },
            Inline::Sized(sized) => RenderInline::Sized {
                level: sized.level(),
                content: self.resolve_inlines(&sized.content()),
            },
            Inline::Link(link) => {
                let display = link
                    .display()
                    .as_ref()
                    .map(|display| self.resolve_inlines(display));
                let written = self.fill(&link.target());
                if is_external_url(&written) {
                    RenderInline::ExternalLink {
                        url: written,
                        display,
                    }
                } else {
                    let title = self.resolve_link_target(&written);
                    // 틀 인자가 비면 앵커도 빈다. the seed는 그때 `#`를 붙이지 않는다.
                    let anchor = self
                        .fill_option(&link.anchor())
                        .filter(|anchor| !anchor.is_empty())
                        .map(|anchor| self.qualify_anchor(&title, anchor));
                    RenderInline::DocumentLink {
                        kind: self.link_kind(&title, anchor.as_deref()),
                        // 표시부가 없으면 적힌 그대로가 글자다 — 해석한 문서명이 아니다
                        // (렌더확정: `[[/심화]]` → `/심화`, `[[#개요]]` → `#개요`).
                        display: display.unwrap_or_else(|| {
                            vec![RenderInline::Text(written_link(&written, &anchor))]
                        }),
                        title,
                        anchor,
                    }
                }
            }
            Inline::Image(image) => {
                let mut layout = ImageLayout::default();
                for option in &image.options() {
                    let Some(value) = self.fill_option(&option.value) else {
                        continue;
                    };
                    let value = value.as_str();
                    match option.name.as_str() {
                        "width" => layout.width = Some(Dimension::parse_image(value)),
                        "height" => layout.height = Some(Dimension::parse_image(value)),
                        "align" => {
                            layout.align = match value.trim() {
                                "left" => Some(ImageAlignment::Left),
                                "center" => Some(ImageAlignment::Center),
                                "right" => Some(ImageAlignment::Right),
                                _ => None,
                            }
                        }
                        "bgcolor" => layout.background_color = ColorValue::parse(value),
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
                // 틀 인자를 채운 뒤 다시 공백을 다듬는다 — 빈 인자(`[[파일:@행정구@ 이름.svg]]`
                // 에서 행정구 미지정)가 남긴 앞 공백은 파일 이름이 아니다(렌더확정: the seed는
                // `파일:이름.svg`로 이미지를 찾는다). 파싱 시점 trim은 채우기 전이라 못 잡는다.
                let file_name = self.fill(&image.file_name()).trim().to_string();
                RenderInline::Image {
                    url: self.context.file_url(&file_name),
                    file_name,
                    layout,
                }
            }
            Inline::WikiStyle(wiki_style) => RenderInline::WikiStyle {
                style: self.fill_style(&wiki_style.style()),
                dark_style: self.fill_style(&wiki_style.dark_style()),
                blocks: self.resolve_blocks(&wiki_style.blocks()),
            },
            Inline::Folding(folding) => RenderInline::Folding {
                summary: self.fill(&folding.summary()),
                blocks: self.resolve_blocks(&folding.blocks()),
            },
            // 조건식은 값을 내면서 변수도 만든다. 만들어진 변수는 뒤따르는
            // `#!if`와 `@이름@`이 함께 쓰므로 스코프에 남긴다.
            // `#!if`는 resolve_inlines가 스코프를 다루며 처리한다.
            Inline::Conditional(_) => return None,
            Inline::CodeBlock(code_block) => RenderInline::CodeBlock {
                language: code_block.language.clone(),
                source: code_block.source.clone(),
            },
            Inline::Html(html) => RenderInline::Html(self.fill(html)),
            // 텍스트 문맥의 `@이름@`. 값이 없으면 아무것도 남기지 않는다.
            Inline::Variable(variable) => RenderInline::Text(
                self.scope
                    .get(&variable.name)
                    .cloned()
                    .or_else(|| variable.default.clone())
                    .unwrap_or_default(),
            ),
            Inline::Category(category) => {
                let name = category.name();
                if !self.categories.contains(&name) {
                    self.categories.push(name);
                }
                return None;
            }
            Inline::Footnote(footnote) => RenderInline::Footnote {
                name: footnote.name(),
                content: self.resolve_inlines(&footnote.content()),
            },
            Inline::Macro(macro_call) => {
                let argument = self.fill_option(&macro_call.argument());
                let range = macro_call.syntax().text_range();
                self.resolve_macro(&macro_call.name(), argument.as_deref(), range)
            }
        })
    }

    fn resolve_styled(&mut self, style: TextStyle, content: &[Inline]) -> RenderInline {
        RenderInline::Styled {
            style,
            content: self.resolve_inlines(content),
        }
    }

    /// 인식하는 매크로인데 인자가 없거나 잘못돼 특화하지 못했음을 알린다.
    /// 편집 중 문서 자신의 지점만 담도록 include 내부에서는 방출하지 않는다.
    fn report_invalid_argument(&mut self, name: &str, range: TextRange) {
        if self.include_instance.is_some() {
            return;
        }
        self.diagnostics.push(Diagnostic {
            code: DiagnosticCode::InvalidMacroArgument,
            range,
            message: format!("매크로 `{name}`의 인자가 없거나 잘못되어 표기 그대로 남습니다."),
            suggestion: None,
        });
    }

    fn resolve_macro(
        &mut self,
        name: &str,
        argument: Option<&str>,
        range: TextRange,
    ) -> RenderInline {
        let unresolved = || RenderInline::Unresolved {
            name: name.to_string(),
            argument: argument.map(str::to_string),
        };
        match name.to_ascii_lowercase().as_str() {
            "목차" | "tableofcontents" => RenderInline::TableOfContents {
                entries: Vec::new(),
            },
            "각주" | "footnote" => RenderInline::FootnoteSection { notes: Vec::new() },
            "br" => RenderInline::LineBreak,
            "clearfix" => RenderInline::ClearFix,
            "anchor" => match argument {
                Some(anchor_name) => RenderInline::Anchor {
                    name: anchor_name.to_string(),
                },
                None => {
                    self.report_invalid_argument(name, range);
                    unresolved()
                }
            },
            "math" => match argument {
                Some(formula) => RenderInline::Math {
                    formula: formula.to_string(),
                },
                None => {
                    self.report_invalid_argument(name, range);
                    unresolved()
                }
            },
            // now()가 없어 원문 표기로 남는 것은 렌더 결정성 정책이지 저자 잘못이
            // 아니다 — 아래 date·age·dday는 인자 자체가 잘못일 때만 진단한다.
            "date" | "datetime" => match self.context.now() {
                Some(now) => RenderInline::Text(now.to_string()),
                None => unresolved(),
            },
            "age" => {
                let birth = argument.and_then(parse_date);
                if birth.is_none() {
                    self.report_invalid_argument(name, range);
                }
                match (birth, self.context.now()) {
                    (Some(birth), Some(now)) => {
                        let today = now.date;
                        let mut age = today.year - birth.year;
                        if (today.month, today.day) < (birth.month, birth.day) {
                            age -= 1;
                        }
                        RenderInline::Text(age.to_string())
                    }
                    _ => unresolved(),
                }
            }
            "dday" => {
                let target = argument.and_then(parse_date);
                if target.is_none() {
                    self.report_invalid_argument(name, range);
                }
                match (target, self.context.now()) {
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
                }
            }
            "youtube" => {
                self.resolve_video(VideoProvider::Youtube, argument, name, range, unresolved)
            }
            "kakaotv" => {
                self.resolve_video(VideoProvider::KakaoTv, argument, name, range, unresolved)
            }
            "nicovideo" => {
                self.resolve_video(VideoProvider::NicoVideo, argument, name, range, unresolved)
            }
            "ruby" => match argument.and_then(parse_ruby) {
                Some(ruby) => ruby,
                None => {
                    self.report_invalid_argument(name, range);
                    unresolved()
                }
            },
            _ => {
                if self.include_instance.is_none() {
                    self.diagnostics.push(Diagnostic {
                        code: DiagnosticCode::UnsupportedMacro,
                        range,
                        message: format!("매크로 `{name}`을(를) 인식하지 못했습니다."),
                        suggestion: None,
                    });
                }
                unresolved()
            }
        }
    }

    fn resolve_video(
        &mut self,
        provider: VideoProvider,
        argument: Option<&str>,
        name: &str,
        range: TextRange,
        unresolved: impl Fn() -> RenderInline,
    ) -> RenderInline {
        let Some(argument) = argument else {
            self.report_invalid_argument(name, range);
            return unresolved();
        };
        let mut parts = argument.split(',');
        let identifier = parts.next().unwrap_or_default().trim().to_string();
        if identifier.is_empty() {
            self.report_invalid_argument(name, range);
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

    fn expand_include(&mut self, argument: Option<&str>, range: TextRange) -> Vec<RenderBlock> {
        let unresolved = |argument: Option<&str>| {
            vec![RenderBlock::Paragraph(vec![RenderInline::Unresolved {
                name: "include".to_string(),
                argument: argument.map(str::to_string),
            }])]
        };
        // 나무위키는 틀 속의 틀(중첩 include)을 확장하지 않는다. 원문을 노출하지도 않고
        // 조용히 버린다 — 틀 문서 끝의 `[include(틀:X/설명 문서)]`가 그 틀을 쓴 문서에
        // 딸려오지 않는 것이 이 규칙 때문이다.
        if self.include_instance.is_some() {
            return Vec::new();
        }
        // 여기 이르렀다면 중첩 include가 아니므로(위에서 걸러짐) 최상위 문서다 —
        // 진단 범위가 편집 중 문서 안이라 그대로 방출해도 된다.
        let Some(argument) = argument else {
            self.report_invalid_argument("include", range);
            return unresolved(argument);
        };
        let mut parts = argument.split(',');
        let title = parts.next().unwrap_or_default().trim().to_string();
        if title.is_empty() {
            self.report_invalid_argument("include", range);
            return unresolved(Some(argument));
        }
        let Some(source) = self.context.include_source(&title) else {
            self.diagnostics.push(Diagnostic {
                code: DiagnosticCode::IncludeTargetMissing,
                range,
                message: format!("include 대상 문서 `{title}`이(가) 존재하지 않습니다."),
                suggestion: None,
            });
            return unresolved(Some(argument));
        };

        // 인자는 스코프에 담긴다. 틀 본문의 `@이름@`을 resolve가 이 스코프로 채운다 —
        // 원문을 미리 치환하지 않으므로 파싱은 한 번뿐이다.
        let mut scope: HashMap<String, String> = parts
            .filter_map(|part| part.split_once('='))
            .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
            .collect();
        // 틀은 호출한 쪽 문서명을 `calleeTitle`로 볼 수 있다.
        if let Some(title) = self.context.current_title() {
            scope.insert("calleeTitle".to_string(), title);
        }

        let document = namumark_parser::parse(&source);
        let outer_scope = std::mem::replace(&mut self.scope, scope);
        self.expanded_includes += 1;
        self.include_instance = Some(IncludeInstance(self.expanded_includes));
        let blocks = self.resolve_blocks(&document.blocks());
        self.include_instance = None;
        self.scope = outer_scope;
        blocks
    }
}

/// 표 속성 이름과 확정된 값을 타입화된 스타일 속성으로 옮긴다.
///
/// 색 표기가 아닌 값(틀 인자가 안 채워진 `<bgcolor=@배경색@>` 등)은 통째로 버린다 —
/// 나무위키가 그렇게 한다. 방출되지 않는 `keepall`·`class`도 여기서 사라진다.
fn table_style_property(name: &str, value: Option<&str>) -> Option<TableStyleProperty> {
    // 값은 따옴표로 감쌀 수 있다(렌더확정: `<tablealign="center">`도 the seed에서 center다).
    let value = value.map(|value| value.trim().trim_matches('"'));
    // 색은 듀얼 표기의 라이트 값만 쓴다(표 색의 다크 모드는 후속 과제).
    let color = |value: Option<&str>| {
        let value = value?;
        ColorValue::parse(value.split(',').next().unwrap_or(value))
    };
    match name {
        "bgcolor" => Some(TableStyleProperty::BackgroundColor(color(value)?)),
        "color" => Some(TableStyleProperty::Color(color(value)?)),
        "bordercolor" => Some(TableStyleProperty::BorderColor(color(value)?)),
        "width" => Some(TableStyleProperty::Width(Dimension::parse(value?))),
        "height" => Some(TableStyleProperty::Height(Dimension::parse(value?))),
        // 나무위키는 left·center·right만 받는다. 그 외 값은 색처럼 선언을 통째로 버린다.
        "textalign" => Some(TableStyleProperty::TextAlign(match value? {
            "left" => HorizontalAlignment::Left,
            "center" => HorizontalAlignment::Center,
            "right" => HorizontalAlignment::Right,
            _ => return None,
        })),
        // 명시한 left·center·right는 각각 정렬 클래스를 만든다(렌더확정: `<tablealign=left>`가
        // the seed에서 `table-left`다). 인식 못한 값은 색처럼 선언을 통째로 버린다 — 정렬을
        // 아예 지정하지 않은 기본과 같아진다.
        "align" => Some(TableStyleProperty::Align(match value? {
            "left" => HorizontalAlignment::Left,
            "center" => HorizontalAlignment::Center,
            "right" => HorizontalAlignment::Right,
            _ => return None,
        })),
        "nopad" => Some(TableStyleProperty::NoPadding),
        _ => None,
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

/// 표시부 없는 링크에 나오는 글자. 적힌 대상 그대로이고 앵커는 빠진다
/// (렌더확정: `[[알파위키#기능]]` → `알파위키`). 대상이 없으면 앵커가 곧 글자다
/// (`[[#개요]]` → `#개요`).
fn written_link(target: &str, anchor: &Option<String>) -> String {
    match (target, anchor) {
        ("", Some(anchor)) => format!("#{anchor}"),
        _ => target.to_string(),
    }
}

fn parse_ruby(argument: &str) -> Option<RenderInline> {
    let (content, options) = argument.split_once(',')?;
    let option = |name: &str| {
        options
            .split(',')
            .find_map(|part| part.trim().strip_prefix(name))
            .map(str::trim)
    };
    Some(RenderInline::Ruby {
        content: content.trim().to_string(),
        ruby: option("ruby=")?.to_string(),
        color: option("color=").and_then(ColorValue::parse),
    })
}

/// 분류·`[include]`만 있는 줄은 화면에 아무것도 남기지 않는다 — 그런 줄로만 뒤가
/// 채워지면 그 앞 개행까지 `<br>`이 되지 않고 사라진다(렌더 증거: 원문
/// `[[분류:X]]\n[include(틀:Y)]\n[목차]\n[clearfix]`를 the seed는
/// `<div class='wiki-paragraph'><목차><br><clearfix></div>`로 낸다). [`Resolver::resolve_paragraph`]가 쓴다.
/// include 확장 결과를 인라인 문맥에 이어 붙인다. 문단은 속내용만 펴고(문단 사이는 br),
/// 표·리스트 등은 감쌀 요소 없이 블록째 담는다. `resolve_blocks_as_inlines`의 RenderBlock판.
fn flatten_render_blocks(blocks: Vec<RenderBlock>) -> Vec<RenderInline> {
    let mut inlines = Vec::new();
    for block in blocks {
        match block {
            RenderBlock::Paragraph(content) => {
                if !inlines.is_empty() {
                    inlines.push(RenderInline::LineBreak);
                }
                inlines.extend(content);
            }
            other => inlines.push(RenderInline::Blocks(vec![other])),
        }
    }
    inlines
}

fn is_invisible_line(line: &[Inline]) -> bool {
    !line.is_empty()
        && line.iter().all(|inline| match inline {
            Inline::Category(_) => true,
            Inline::Macro(macro_call) => macro_call.name().eq_ignore_ascii_case("include"),
            _ => false,
        })
}
