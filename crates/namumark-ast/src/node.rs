//! 무손실 구문 트리 위의 타입 뷰.
//!
//! 각 뷰는 `SyntaxNode` 하나만 감싸고(소유 데이터 없음), 접근자가 토큰을 읽어
//! 의미값을 계산한다. 경계 나누기는 문법 계층이 이미 끝냈으므로 여기서는 문법이 끊어
//! 놓은 토큰을 집어 단일 토큰 값만 해석한다 — 원문을 다시 쪼개지 않는다.

use crate::value::{
    Fragment, HorizontalAlignment, ImageOption, ListKind, TableAttribute, TableAttributeScope,
    Template, Variable, VerticalAlignment,
};
use namumark_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};
use namumark_text as text;

pub trait AstNode: Sized {
    fn cast(syntax: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

macro_rules! ast_node {
    ($name:ident, $($kind:ident)|+) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            syntax: SyntaxNode,
        }
        impl AstNode for $name {
            fn cast(syntax: SyntaxNode) -> Option<Self> {
                matches!(syntax.kind(), $(SyntaxKind::$kind)|+).then_some(Self { syntax })
            }
            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

// ---- 문서 ----

ast_node!(Document, Document);

impl Document {
    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

/// 나무마크 원문을 문서 뷰로 파싱한다.
pub fn parse(source: &str) -> Document {
    Document::cast(namumark_syntax::parse(source).root()).expect("루트는 언제나 Document 노드다")
}

// ---- 블록 ----

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(Heading),
    Paragraph(Paragraph),
    HorizontalRule,
    Quote(Quote),
    List(List),
    Indent(Indent),
    Table(Table),
    Comment(Comment),
    Redirect(Redirect),
}

impl Block {
    fn cast(syntax: SyntaxNode) -> Option<Block> {
        Some(match syntax.kind() {
            SyntaxKind::Heading => Block::Heading(Heading { syntax }),
            SyntaxKind::Paragraph => Block::Paragraph(Paragraph { syntax }),
            SyntaxKind::HorizontalRule => Block::HorizontalRule,
            SyntaxKind::Quote => Block::Quote(Quote { syntax }),
            SyntaxKind::List => Block::List(List { syntax }),
            SyntaxKind::Indent => Block::Indent(Indent { syntax }),
            SyntaxKind::Table => Block::Table(Table { syntax }),
            SyntaxKind::Comment => Block::Comment(Comment { syntax }),
            SyntaxKind::Redirect => Block::Redirect(Redirect { syntax }),
            _ => return None,
        })
    }
}

ast_node!(Heading, Heading);
ast_node!(Paragraph, Paragraph);
ast_node!(Quote, Quote);
ast_node!(Indent, Indent);
ast_node!(List, List);
ast_node!(ListItem, ListItem);
ast_node!(Table, Table);
ast_node!(Comment, Comment);
ast_node!(Redirect, Redirect);

impl Heading {
    /// 여는 `==`/`==#`의 `=` 개수.
    pub fn level(&self) -> u8 {
        marker_text(&self.syntax)
            .bytes()
            .filter(|&byte| byte == b'=')
            .count() as u8
    }

    /// 여는 표식에 `#`이 있으면 접힘.
    pub fn folded(&self) -> bool {
        marker_text(&self.syntax).contains('#')
    }

    pub fn content(&self) -> Vec<Inline> {
        inlines(&self.syntax)
    }
}

impl Paragraph {
    pub fn inlines(&self) -> Vec<Inline> {
        inlines(&self.syntax)
    }
}

impl Quote {
    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

impl Indent {
    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

impl List {
    pub fn kind(&self) -> ListKind {
        self.items()
            .first()
            .map(ListItem::kind)
            .unwrap_or(ListKind::Unordered)
    }

    pub fn items(&self) -> Vec<ListItem> {
        self.syntax.children().filter_map(ListItem::cast).collect()
    }
}

impl ListItem {
    /// 여러 줄 항목은 마커를 하위 영역의 줄머리로 옮기므로 자손까지 본다.
    /// 종류(`1.`) 바로 뒤에 시작번호(`#42`)가 오면 이어 붙여 `1.#42`로 되짚는다.
    fn marker_text(&self) -> String {
        let mut tokens = self
            .syntax
            .descendants_with_tokens()
            .filter_map(NodeOrToken::into_token);
        match tokens.find(|token| token.kind() == SyntaxKind::ListMarker) {
            Some(bullet) => {
                let mut text = bullet.text().to_string();
                if let Some(number) = tokens.next()
                    && number.kind() == SyntaxKind::ListStartNumber
                {
                    text.push_str(number.text());
                }
                text
            }
            None => String::new(),
        }
    }

    fn kind(&self) -> ListKind {
        match text::list_marker(&self.marker_text()) {
            Some(marker) => list_kind(marker.kind),
            None => ListKind::Unordered,
        }
    }

    /// `1.#42`처럼 순서 리스트의 번호를 재지정하는 경우에만 값이 있다.
    pub fn start_number(&self) -> Option<u32> {
        text::list_marker(&self.marker_text()).and_then(|marker| marker.start_number)
    }

    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

impl Comment {
    pub fn text(&self) -> String {
        let line = raw_text_tokens(&self.syntax);
        line.strip_prefix("##").unwrap_or(&line).to_string()
    }
}

impl Redirect {
    /// 지시자(`#redirect `)는 별도 토큰이라 Text에는 대상만 남는다.
    pub fn target(&self) -> Template {
        template_of(raw_text_tokens(&self.syntax).trim())
    }
}

// ---- 표 ----

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCell {
    /// 지정하지 않았으면 None (기본 1, `colspan`·`rowspan` 미방출).
    pub column_span: Option<u32>,
    pub row_span: Option<u32>,
    /// 정렬을 지정하지 않은 셀은 None이다.
    pub horizontal_alignment: Option<HorizontalAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub attributes: Vec<TableAttribute>,
    pub blocks: Vec<Block>,
}

impl Table {
    pub fn caption(&self) -> Option<Vec<Inline>> {
        // 캡션은 첫 행의 직접 자식이다. descendants로 넓히면 셀 안 중첩 표의 캡션까지 집는다.
        self.syntax
            .children()
            .filter(|row| row.kind() == SyntaxKind::TableRow)
            .flat_map(|row| row.children())
            .find(|node| node.kind() == SyntaxKind::TableCaption)
            .map(|caption| inlines(&caption))
    }

    pub fn rows(&self) -> Vec<TableRow> {
        self.syntax
            .children()
            .filter(|child| child.kind() == SyntaxKind::TableRow)
            .map(|row| table_row(&row))
            .collect()
    }
}

fn table_row(node: &SyntaxNode) -> TableRow {
    let mut cells = Vec::new();
    // 다음 셀의 자동 colspan = 직전 파이프 런의 쌍 수. 캡션은 가상 `||` 한 쌍을 더한다.
    let mut pending_pairs = 0usize;
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) => {
                if token.kind() == SyntaxKind::Separator {
                    let text = token.text();
                    if !text.is_empty() && text.bytes().all(|byte| byte == b'|') {
                        pending_pairs += text.len() / 2;
                    }
                }
            }
            NodeOrToken::Node(child) => match child.kind() {
                SyntaxKind::TableCaption => pending_pairs += 1,
                SyntaxKind::TableCell => {
                    cells.push(table_cell(&child, pending_pairs));
                    pending_pairs = 0;
                }
                _ => {}
            },
        }
    }
    TableRow { cells }
}

/// 옵션·정렬을 셀 노드에서 계산한다.
///
/// 마커 배치 규칙(문법 계층과의 계약): 내용 노드 앞의 `<…>` 옵션 토큰은 옵션이고,
/// AlignmentSpace 토큰은 내용 앞뒤의 정렬 결정 공백이다.
fn table_cell(node: &SyntaxNode, pending_pairs: usize) -> TableCell {
    let mut semantics = CellSemantics::default();
    let mut leading_space = false;
    let mut trailing_space = false;
    let mut seen_content = false;
    // `<…>` 옵션은 여는 `<`(DelimiterOpen)부터 닫는 `>`(DelimiterClose)까지 한 그룹이다.
    let mut option_inner = String::new();
    let mut in_option = false;
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) => match token.kind() {
                SyntaxKind::AlignmentSpace => {
                    if seen_content {
                        trailing_space = true;
                    } else {
                        leading_space = true;
                    }
                }
                _ if seen_content => {}
                SyntaxKind::DelimiterOpen => {
                    in_option = true;
                    option_inner.clear();
                }
                SyntaxKind::DelimiterClose => {
                    if in_option && let Some(option) = text::cell_option(&option_inner) {
                        semantics.absorb(&option);
                    }
                    in_option = false;
                }
                SyntaxKind::CellOptionName
                | SyntaxKind::CellOptionValue
                | SyntaxKind::CellOption
                | SyntaxKind::Separator => option_inner.push_str(token.text()),
                _ => {}
            },
            NodeOrToken::Node(_) => seen_content = true,
        }
    }

    // 나무위키는 정렬을 **지정한** 셀에만 text-align을 방출한다.
    let horizontal_alignment = semantics.horizontal_alignment.or({
        if leading_space && trailing_space {
            Some(HorizontalAlignment::Center)
        } else if leading_space {
            Some(HorizontalAlignment::Right)
        } else if trailing_space {
            Some(HorizontalAlignment::Left)
        } else {
            None
        }
    });
    TableCell {
        // 지정한 대로만 싣는다 — the seed는 `<-1>`로 적힌 1도 `colspan='1'`로 낸다.
        column_span: semantics
            .column_span_override
            .or_else(|| (pending_pairs > 1).then_some(pending_pairs as u32)),
        row_span: semantics.row_span,
        horizontal_alignment,
        vertical_alignment: semantics.vertical_alignment,
        attributes: semantics.attributes,
        blocks: block_children(node),
    }
}

#[derive(Default)]
struct CellSemantics {
    column_span_override: Option<u32>,
    row_span: Option<u32>,
    horizontal_alignment: Option<HorizontalAlignment>,
    vertical_alignment: Option<VerticalAlignment>,
    attributes: Vec<TableAttribute>,
}

impl CellSemantics {
    fn absorb(&mut self, option: &text::CellOption<'_>) {
        match option {
            text::CellOption::Alignment(alignment) => {
                self.horizontal_alignment = Some(horizontal_alignment(*alignment));
            }
            text::CellOption::ColumnSpan(span) => self.column_span_override = Some(*span),
            text::CellOption::RowSpan {
                span,
                vertical_position,
            } => {
                self.row_span = Some(*span);
                if let Some(vertical_position) = vertical_position {
                    self.vertical_alignment = Some(match vertical_position {
                        text::VerticalPosition::Top => VerticalAlignment::Top,
                        text::VerticalPosition::Bottom => VerticalAlignment::Bottom,
                    });
                }
            }
            text::CellOption::Flag { scope, name } => self.attributes.push(TableAttribute {
                scope: attribute_scope(*scope, self.column_span_override),
                name: (*name).to_string(),
                value: None,
            }),
            text::CellOption::Attribute { scope, name, value } => {
                self.attributes.push(TableAttribute {
                    scope: attribute_scope(*scope, self.column_span_override),
                    name: (*name).to_string(),
                    value: Some(template_of(value)),
                });
            }
            text::CellOption::BackgroundColor(value) => self.attributes.push(TableAttribute {
                scope: TableAttributeScope::Cell,
                name: "bgcolor".to_string(),
                value: Some(template_of(value)),
            }),
        }
    }
}

// ---- 인라인 ----

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    LineBreak,
    Bold(Bold),
    Italic(Italic),
    Strikethrough(Strikethrough),
    Underline(Underline),
    Superscript(Superscript),
    Subscript(Subscript),
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

ast_node!(Bold, Bold);
ast_node!(Italic, Italic);
ast_node!(Strikethrough, Strikethrough);
ast_node!(Underline, Underline);
ast_node!(Superscript, Superscript);
ast_node!(Subscript, Subscript);
ast_node!(Link, Link);
ast_node!(Image, Image);
ast_node!(Category, Category);
ast_node!(Footnote, Footnote);
ast_node!(Macro, MacroCall);
ast_node!(ColoredText, ColoredText | ColoredBlock);
ast_node!(SizedText, SizedText | SizedBlock);
ast_node!(WikiStyle, WikiStyle);
ast_node!(Folding, Folding);
ast_node!(Conditional, Conditional);

macro_rules! styled {
    ($name:ident) => {
        impl $name {
            pub fn content(&self) -> Vec<Inline> {
                inlines(&self.syntax)
            }
        }
    };
}
styled!(Bold);
styled!(Italic);
styled!(Strikethrough);
styled!(Underline);
styled!(Superscript);
styled!(Subscript);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub source: String,
}

impl Link {
    pub fn target(&self) -> Template {
        let raw = first_token_text(&self.syntax, SyntaxKind::LinkTarget).unwrap_or_default();
        // `[[:파일:…]]`의 선행 `:`는 이름공간을 본문 링크로 강등하는 표시라 이름에서 뗀다.
        let stripped = match raw.strip_prefix(':') {
            Some(rest)
                if text::strip_link_prefix(rest, &["파일:", "file:", "분류:", "category:"])
                    .is_some() =>
            {
                rest
            }
            _ => &raw,
        };
        template_of(&text::unescape(stripped))
    }

    /// `[[문서#s-1]]`의 `s-1`. 외부 URL은 분리하지 않는다.
    pub fn anchor(&self) -> Option<Template> {
        first_token_text(&self.syntax, SyntaxKind::LinkAnchor)
            .map(|anchor| template_of(&text::unescape(&anchor)))
    }

    pub fn display(&self) -> Option<Vec<Inline>> {
        // 표시부 구분자 `|`가 있으면 표시부가 있다(`#` 앵커 구분자와 텍스트로 구별).
        let has_display = self
            .syntax
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .any(|token| token.kind() == SyntaxKind::Separator && token.text() == "|");
        has_display.then(|| inlines(&self.syntax))
    }
}

impl Image {
    pub fn file_name(&self) -> Template {
        // `파일:` 접두 뒤 공백은 파일 이름이 아니다 — 이름공간은 별도 토큰이라 LinkTarget엔 이름만 남는다.
        let name = first_token_text(&self.syntax, SyntaxKind::LinkTarget).unwrap_or_default();
        template_of(name.trim())
    }

    pub fn options(&self) -> Vec<ImageOption> {
        let mut options = Vec::new();
        let mut tokens = self
            .syntax
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token);
        while let Some(token) = tokens.next() {
            match token.kind() {
                SyntaxKind::ArgumentName => {
                    let name = token.text().trim().to_string();
                    // 이름 뒤에는 `=`(Separator)와 값(ArgumentValue)이 따른다.
                    tokens.next();
                    let value = tokens
                        .next()
                        .map(|value| template_of(value.text().trim()))
                        .unwrap_or_default();
                    if !name.is_empty() {
                        options.push(ImageOption {
                            name,
                            value: Some(value),
                        });
                    }
                }
                SyntaxKind::MacroArgument => {
                    let name = token.text().trim().to_string();
                    if !name.is_empty() {
                        options.push(ImageOption { name, value: None });
                    }
                }
                _ => {}
            }
        }
        options
    }
}

impl Category {
    pub fn name(&self) -> String {
        first_token_text(&self.syntax, SyntaxKind::LinkTarget).unwrap_or_default()
    }
}

impl Footnote {
    pub fn name(&self) -> Option<String> {
        first_token_text(&self.syntax, SyntaxKind::FootnoteName).filter(|name| !name.is_empty())
    }

    pub fn content(&self) -> Vec<Inline> {
        inlines(&self.syntax)
    }
}

impl Macro {
    pub fn name(&self) -> String {
        first_token_text(&self.syntax, SyntaxKind::MacroName).unwrap_or_default()
    }

    /// 여는 `(`와 닫는 `)` 사이 원문. 괄호가 없으면 None.
    pub fn argument(&self) -> Option<Template> {
        let mut after_name = self
            .syntax
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .skip_while(|token| token.kind() != SyntaxKind::MacroName);
        after_name.next();
        // 남은 토큰은 `(` … `)` `]`. 여는·닫는 괄호와 `]`를 제외한 사이가 인자다.
        let inner: Vec<SyntaxToken> = after_name
            .filter(|token| token.kind() != SyntaxKind::DelimiterClose)
            .collect();
        if inner.is_empty() {
            return None;
        }
        let argument: String = inner[1..inner.len().saturating_sub(1)]
            .iter()
            .map(|token| token.text())
            .collect();
        Some(template_of(&argument))
    }
}

impl ColoredText {
    fn colors(&self) -> Option<(String, Option<String>)> {
        let value = first_token_text(&self.syntax, SyntaxKind::ColorValue)?;
        text::parse_color_specification(&value)
    }

    pub fn color(&self) -> String {
        self.colors().map(|(color, _)| color).unwrap_or_default()
    }

    pub fn dark_color(&self) -> Option<String> {
        self.colors().and_then(|(_, dark)| dark)
    }

    pub fn content(&self) -> Vec<Inline> {
        match self.syntax.kind() {
            SyntaxKind::ColoredBlock => block_children_as_inlines(&self.syntax),
            _ => inlines(&self.syntax),
        }
    }
}

impl SizedText {
    pub fn level(&self) -> i8 {
        first_token_text(&self.syntax, SyntaxKind::SizeLevel)
            .and_then(|value| text::parse_size_marker(&value).map(|(level, _)| level))
            .unwrap_or(0)
    }

    pub fn content(&self) -> Vec<Inline> {
        match self.syntax.kind() {
            SyntaxKind::SizedBlock => block_children_as_inlines(&self.syntax),
            _ => inlines(&self.syntax),
        }
    }
}

impl WikiStyle {
    pub fn style(&self) -> Option<Template> {
        self.attribute_value("style")
    }

    pub fn dark_style(&self) -> Option<Template> {
        self.attribute_value("dark-style")
    }

    /// 같은 이름의 속성이 여럿이면 값을 이어 붙인다(parse_wiki_style_attributes와 동일).
    fn attribute_value(&self, name: &str) -> Option<Template> {
        let mut value = String::new();
        let mut found = false;
        let mut tokens = self
            .syntax
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token);
        while let Some(token) = tokens.next() {
            if token.kind() == SyntaxKind::AttributeName && token.text() == name {
                tokens.next(); // `=` Separator
                if let Some(raw) = tokens.next()
                    && raw.kind() == SyntaxKind::AttributeValue
                {
                    value.push_str(strip_quotes(raw.text()));
                    found = true;
                }
            }
        }
        found.then(|| template_of(&value))
    }

    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

impl Folding {
    /// 접기 문구. 위키 문법이 적용되지 않아 글자 그대로다.
    pub fn summary(&self) -> Template {
        let summary = self
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::FoldingSummary)
            .map(|summary| summary.text().to_string())
            .unwrap_or_default();
        template_of(&summary)
    }

    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

impl Conditional {
    pub fn expression(&self) -> String {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ConditionExpression)
            .map(|expression| expression.text().to_string())
            .unwrap_or_default()
    }

    pub fn blocks(&self) -> Vec<Block> {
        block_children(&self.syntax)
    }
}

fn cast_inline(node: SyntaxNode) -> Option<Inline> {
    Some(match node.kind() {
        SyntaxKind::Bold => Inline::Bold(Bold { syntax: node }),
        SyntaxKind::Italic => Inline::Italic(Italic { syntax: node }),
        SyntaxKind::Strikethrough => Inline::Strikethrough(Strikethrough { syntax: node }),
        SyntaxKind::Underline => Inline::Underline(Underline { syntax: node }),
        SyntaxKind::Superscript => Inline::Superscript(Superscript { syntax: node }),
        SyntaxKind::Subscript => Inline::Subscript(Subscript { syntax: node }),
        SyntaxKind::Literal => Inline::Literal(raw_text_tokens(&node)),
        SyntaxKind::InlineHtml => Inline::Html(template_of(&raw_text_tokens(&node))),
        SyntaxKind::HtmlBlock => Inline::Html(template_of(&raw_content_text(&node))),
        SyntaxKind::Link => Inline::Link(Link { syntax: node }),
        SyntaxKind::Image => Inline::Image(Image { syntax: node }),
        SyntaxKind::Category => Inline::Category(Category { syntax: node }),
        SyntaxKind::Footnote => Inline::Footnote(Footnote { syntax: node }),
        SyntaxKind::MacroCall => Inline::Macro(Macro { syntax: node }),
        SyntaxKind::TemplateVariable => Inline::Variable(template_variable(&node)?),
        SyntaxKind::ColoredText | SyntaxKind::ColoredBlock => {
            Inline::Colored(ColoredText { syntax: node })
        }
        SyntaxKind::SizedText | SyntaxKind::SizedBlock => Inline::Sized(SizedText { syntax: node }),
        SyntaxKind::WikiStyle => Inline::WikiStyle(WikiStyle { syntax: node }),
        SyntaxKind::Folding => Inline::Folding(Folding { syntax: node }),
        SyntaxKind::Conditional => Inline::Conditional(Conditional { syntax: node }),
        SyntaxKind::CodeBlock => Inline::CodeBlock(code_block(&node)),
        _ => return None,
    })
}

fn template_variable(node: &SyntaxNode) -> Option<Variable> {
    let name = first_token_text(node, SyntaxKind::VariableName)?;
    Some(Variable {
        name,
        default: first_token_text(node, SyntaxKind::VariableDefault),
    })
}

fn code_block(node: &SyntaxNode) -> CodeBlock {
    let language =
        first_token_text(node, SyntaxKind::CodeLanguage).filter(|language| !language.is_empty());
    CodeBlock {
        language,
        source: raw_content_text(node),
    }
}

// ---- 공통 순회·추출 ----

fn block_children(node: &SyntaxNode) -> Vec<Block> {
    node.children().filter_map(Block::cast).collect()
}

/// `{{{#색상}}}`·`{{{+N}}}`이 여러 줄에 걸친 경우의 내용. 서식일 뿐이라 안쪽 블록을 인라인으로 편다.
fn block_children_as_inlines(node: &SyntaxNode) -> Vec<Inline> {
    let mut result = Vec::new();
    for block in block_children(node) {
        if let Block::Paragraph(paragraph) = block {
            if !result.is_empty() {
                result.push(Inline::LineBreak);
            }
            result.extend(paragraph.inlines());
        }
        // 서식 그룹 안의 표·리스트는 인라인으로 펼 수 없다. 드문 형태라 버린다.
    }
    result
}

fn inlines(node: &SyntaxNode) -> Vec<Inline> {
    let mut result = Vec::new();
    let mut buffer = String::new();
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) => match token.kind() {
                SyntaxKind::Text => buffer.push_str(token.text()),
                SyntaxKind::Escaped => buffer.push_str(&token.text()[1..]),
                SyntaxKind::Newline => {
                    flush_text(&mut buffer, &mut result);
                    result.push(Inline::LineBreak);
                }
                _ => {}
            },
            NodeOrToken::Node(child) => {
                if let Some(inline) = cast_inline(child) {
                    flush_text(&mut buffer, &mut result);
                    result.push(inline);
                }
            }
        }
    }
    flush_text(&mut buffer, &mut result);
    result
}

fn flush_text(buffer: &mut String, result: &mut Vec<Inline>) {
    if !buffer.is_empty() {
        result.push(Inline::Text(std::mem::take(buffer)));
    }
}

/// 노드 첫머리의 표식 토큰(헤딩·리스트 등의 여는 표식).
fn marker_text(node: &SyntaxNode) -> String {
    first_token_text(node, SyntaxKind::DelimiterOpen).unwrap_or_default()
}

fn first_token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .find(|token| token.kind() == kind)
        .map(|token| token.text().to_string())
}

/// Text 토큰만 이어붙인다 (Comment/Redirect/Literal처럼 한 줄짜리 원문 복원용).
fn raw_text_tokens(node: &SyntaxNode) -> String {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(|token| token.kind() == SyntaxKind::Text)
        .map(|token| token.text().to_string())
        .collect()
}

/// CodeBlock/HtmlBlock의 원문 복원: Text 토큰과 개행만 모으고 가장자리 개행 하나씩 제거.
fn raw_content_text(node: &SyntaxNode) -> String {
    let mut content = String::new();
    for element in node.children_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            match token.kind() {
                SyntaxKind::Text => content.push_str(token.text()),
                SyntaxKind::Newline => content.push('\n'),
                _ => {}
            }
        }
    }
    let content = content.strip_prefix('\n').unwrap_or(&content);
    let content = content.strip_suffix('\n').unwrap_or(content);
    content.to_string()
}

fn strip_quotes(value: &str) -> &str {
    let mut chars = value.chars();
    match (chars.next(), chars.next_back()) {
        (Some(open), Some(close)) if (open == '"' || open == '\'') && open == close => {
            &value[open.len_utf8()..value.len() - close.len_utf8()]
        }
        _ => value,
    }
}

// ---- 언어 어휘 매핑 ----

fn list_kind(kind: text::ListMarkerKind) -> ListKind {
    match kind {
        text::ListMarkerKind::Unordered => ListKind::Unordered,
        text::ListMarkerKind::Decimal => ListKind::Decimal,
        text::ListMarkerKind::LowerAlphabet => ListKind::LowerAlphabet,
        text::ListMarkerKind::UpperAlphabet => ListKind::UpperAlphabet,
        text::ListMarkerKind::LowerRoman => ListKind::LowerRoman,
        text::ListMarkerKind::UpperRoman => ListKind::UpperRoman,
    }
}

fn horizontal_alignment(alignment: text::CellAlignment) -> HorizontalAlignment {
    match alignment {
        text::CellAlignment::Left => HorizontalAlignment::Left,
        text::CellAlignment::Center => HorizontalAlignment::Center,
        text::CellAlignment::Right => HorizontalAlignment::Right,
    }
}

fn attribute_scope(scope: text::CellOptionScope, columns: Option<u32>) -> TableAttributeScope {
    match scope {
        text::CellOptionScope::Cell => TableAttributeScope::Cell,
        text::CellOptionScope::Row => TableAttributeScope::Row,
        // 여기까지 아는 칸 수만큼의 열에 걸린다 — 나무위키는 옵션을 왼쪽부터 처리한다.
        text::CellOptionScope::Column => TableAttributeScope::Column {
            columns: columns.unwrap_or(1),
        },
        text::CellOptionScope::Table => TableAttributeScope::Table,
    }
}

// ---- 틀 인자 조각화 ----

/// 토큰 텍스트에서 틀 인자 표기(`@이름@`)를 갈라 `Template`을 만든다.
///
/// 인라인 문맥의 `@이름@`은 구문 트리가 노드로 끊어 주지만, 링크 대상·옵션 값처럼
/// 마커 토큰 하나로 들어오는 문자열은 여기서 갈라낸다(단일 토큰 값 해석).
pub fn template_of(source: &str) -> Template {
    let mut fragments = Vec::new();
    let mut pending = String::new();
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(shape) = text::variable_shape(rest) {
            if !pending.is_empty() {
                fragments.push(Fragment::Text(std::mem::take(&mut pending)));
            }
            fragments.push(Fragment::Variable(Variable {
                name: rest[shape.name.clone()].to_string(),
                default: shape.default.clone().map(|range| rest[range].to_string()),
            }));
            rest = &rest[shape.length..];
            continue;
        }
        let next = rest
            .char_indices()
            .skip(1)
            .find(|(_, character)| *character == '@')
            .map(|(index, _)| index)
            .unwrap_or(rest.len());
        pending.push_str(&rest[..next]);
        rest = &rest[next..];
    }
    if !pending.is_empty() {
        fragments.push(Fragment::Text(pending));
    }
    Template(fragments)
}
