//! 테스트용 소유 모델.
//!
//! 뷰([`namumark_ast`])는 구문 트리를 가리키므로 값 동등성 비교가 안 된다. 이 모듈은
//! 뷰를 소유 트리로 옮겨(`of`) 테스트가 기대값 리터럴과 `assert_eq!`로 겨루게 한다.
//! 옮김은 뷰 접근자를 호출할 뿐이라 별도 파싱을 하지 않는다.

use namumark_ast as ast;
pub use namumark_ast::{
    HorizontalAlignment, ImageOption, ListKind, TableAttribute, Template, VerticalAlignment,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub folded: bool,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    pub kind: ListKind,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub start_number: Option<u32>,
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
    pub column_span: Option<u32>,
    pub row_span: Option<u32>,
    pub horizontal_alignment: Option<HorizontalAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub attributes: Vec<TableAttribute>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub source: String,
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
    Variable(ast::Variable),
    Colored(ColoredText),
    Sized(SizedText),
    WikiStyle(WikiStyle),
    Folding(Folding),
    Conditional(Conditional),
    CodeBlock(CodeBlock),
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
    pub anchor: Option<Template>,
    pub display: Option<Vec<Inline>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub file_name: Template,
    pub options: Vec<ImageOption>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiStyle {
    pub style: Option<Template>,
    pub dark_style: Option<Template>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folding {
    pub summary: Template,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conditional {
    pub expression: String,
    pub blocks: Vec<Block>,
}

// ---- 뷰 → 소유 모델 ----

pub fn of(document: &ast::Document) -> Vec<Block> {
    document.blocks().iter().map(block).collect()
}

fn blocks(blocks: &[ast::Block]) -> Vec<Block> {
    blocks.iter().map(block).collect()
}

fn block(block: &ast::Block) -> Block {
    match block {
        ast::Block::Heading(heading) => Block::Heading(Heading {
            level: heading.level(),
            folded: heading.folded(),
            content: inlines(&heading.content()),
        }),
        ast::Block::Paragraph(paragraph) => Block::Paragraph(inlines(&paragraph.inlines())),
        ast::Block::HorizontalRule => Block::HorizontalRule,
        ast::Block::Quote(quote) => Block::Quote(blocks(&quote.blocks())),
        ast::Block::Indent(indent) => Block::Indent(blocks(&indent.blocks())),
        ast::Block::List(list) => Block::List(List {
            kind: list.kind(),
            items: list
                .items()
                .iter()
                .map(|item| ListItem {
                    start_number: item.start_number(),
                    blocks: blocks(&item.blocks()),
                })
                .collect(),
        }),
        ast::Block::Table(table) => Block::Table(Table {
            caption: table.caption().as_deref().map(inlines),
            rows: table
                .rows()
                .iter()
                .map(|row| TableRow {
                    cells: row.cells.iter().map(table_cell).collect(),
                })
                .collect(),
        }),
        ast::Block::Comment(comment) => Block::Comment(comment.text()),
        ast::Block::Redirect(redirect) => Block::Redirect(redirect.target()),
    }
}

fn table_cell(cell: &ast::TableCell) -> TableCell {
    TableCell {
        column_span: cell.column_span,
        row_span: cell.row_span,
        horizontal_alignment: cell.horizontal_alignment,
        vertical_alignment: cell.vertical_alignment,
        attributes: cell.attributes.clone(),
        blocks: blocks(&cell.blocks),
    }
}

fn inlines(inlines: &[ast::Inline]) -> Vec<Inline> {
    inlines.iter().map(inline).collect()
}

fn inline(inline: &ast::Inline) -> Inline {
    match inline {
        ast::Inline::Text(text) => Inline::Text(text.clone()),
        ast::Inline::LineBreak => Inline::LineBreak,
        ast::Inline::Bold(styled) => Inline::Bold(inlines(&styled.content())),
        ast::Inline::Italic(styled) => Inline::Italic(inlines(&styled.content())),
        ast::Inline::Strikethrough(styled) => Inline::Strikethrough(inlines(&styled.content())),
        ast::Inline::Underline(styled) => Inline::Underline(inlines(&styled.content())),
        ast::Inline::Superscript(styled) => Inline::Superscript(inlines(&styled.content())),
        ast::Inline::Subscript(styled) => Inline::Subscript(inlines(&styled.content())),
        ast::Inline::Literal(text) => Inline::Literal(text.clone()),
        ast::Inline::Link(link) => Inline::Link(Link {
            target: link.target(),
            anchor: link.anchor(),
            display: link.display().as_deref().map(inlines),
        }),
        ast::Inline::Image(image) => Inline::Image(Image {
            file_name: image.file_name(),
            options: image.options(),
        }),
        ast::Inline::Category(category) => Inline::Category(Category {
            name: category.name(),
        }),
        ast::Inline::Footnote(footnote) => Inline::Footnote(Footnote {
            name: footnote.name(),
            content: inlines(&footnote.content()),
        }),
        ast::Inline::Macro(macro_call) => Inline::Macro(Macro {
            name: macro_call.name(),
            argument: macro_call.argument(),
        }),
        ast::Inline::Variable(variable) => Inline::Variable(variable.clone()),
        ast::Inline::Colored(colored) => Inline::Colored(ColoredText {
            color: colored.color(),
            dark_color: colored.dark_color(),
            content: inlines(&colored.content()),
        }),
        ast::Inline::Sized(sized) => Inline::Sized(SizedText {
            level: sized.level(),
            content: inlines(&sized.content()),
        }),
        ast::Inline::WikiStyle(wiki_style) => Inline::WikiStyle(WikiStyle {
            style: wiki_style.style(),
            dark_style: wiki_style.dark_style(),
            blocks: blocks(&wiki_style.blocks()),
        }),
        ast::Inline::Folding(folding) => Inline::Folding(Folding {
            summary: folding.summary(),
            blocks: blocks(&folding.blocks()),
        }),
        ast::Inline::Conditional(conditional) => Inline::Conditional(Conditional {
            expression: conditional.expression(),
            blocks: blocks(&conditional.blocks()),
        }),
        ast::Inline::CodeBlock(code_block) => Inline::CodeBlock(CodeBlock {
            language: code_block.language.clone(),
            source: code_block.source.clone(),
        }),
        ast::Inline::Html(html) => Inline::Html(html.clone()),
    }
}
