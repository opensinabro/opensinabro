use namumark::{
    Block, CodeBlock, ColoredText, Folding, Footnote, Heading, HorizontalAlignment, Inline, Link,
    List, ListItem, ListKind, SizedText, Table, TableCell, TableRow, WikiStyle, parse,
};

fn text(content: &str) -> Inline {
    Inline::Text(content.to_string())
}

fn paragraph(inlines: Vec<Inline>) -> Block {
    Block::Paragraph(inlines)
}

fn simple_cell(content: &str, alignment: HorizontalAlignment) -> TableCell {
    TableCell {
        column_span: 1,
        row_span: 1,
        horizontal_alignment: alignment,
        vertical_alignment: None,
        attributes: vec![],
        blocks: vec![paragraph(vec![text(content)])],
    }
}

fn unordered_list(items: Vec<Vec<Block>>) -> Block {
    Block::List(List {
        kind: ListKind::Unordered,
        items: items
            .into_iter()
            .map(|blocks| ListItem {
                start_number: None,
                blocks,
            })
            .collect(),
    })
}

#[test]
fn quote_containing_table_list_and_paragraph() {
    let document = parse("> || A ||\n>  * 항목\n> 텍스트");
    assert_eq!(
        document.blocks,
        vec![Block::Quote(vec![
            Block::Table(Table {
                caption: None,
                rows: vec![TableRow {
                    cells: vec![simple_cell("A", HorizontalAlignment::Center)],
                }],
            }),
            unordered_list(vec![vec![paragraph(vec![text("항목")])]]),
            paragraph(vec![text("텍스트")]),
        ])]
    );
}

#[test]
fn table_cell_containing_folding_with_inner_table() {
    let document = parse("||<:>첫\n{{{#!folding 보기\n|| 안 || 표 ||\n}}}\n끝 ||");
    assert_eq!(
        document.blocks,
        vec![Block::Table(Table {
            caption: None,
            rows: vec![TableRow {
                cells: vec![TableCell {
                    column_span: 1,
                    row_span: 1,
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: None,
                    attributes: vec![],
                    blocks: vec![
                        paragraph(vec![text("첫")]),
                        Block::Folding(Folding {
                            summary: vec![text("보기")],
                            blocks: vec![Block::Table(Table {
                                caption: None,
                                rows: vec![TableRow {
                                    cells: vec![
                                        simple_cell("안", HorizontalAlignment::Center),
                                        simple_cell("표", HorizontalAlignment::Center),
                                    ],
                                }],
                            })],
                        }),
                        paragraph(vec![text("끝")]),
                    ],
                }],
            }],
        })]
    );
}

#[test]
fn wiki_style_containing_list_and_table() {
    let document = parse("{{{#!wiki\n * 항목\n|| A ||\n}}}");
    assert_eq!(
        document.blocks,
        vec![Block::WikiStyle(WikiStyle {
            style: None,
            dark_style: None,
            blocks: vec![
                unordered_list(vec![vec![paragraph(vec![text("항목")])]]),
                Block::Table(Table {
                    caption: None,
                    rows: vec![TableRow {
                        cells: vec![simple_cell("A", HorizontalAlignment::Center)],
                    }],
                }),
            ],
        })]
    );
}

#[test]
fn nested_folding_blocks() {
    let document = parse("{{{#!folding 바깥\n{{{#!folding 안쪽\n내용\n}}}\n}}}");
    assert_eq!(
        document.blocks,
        vec![Block::Folding(Folding {
            summary: vec![text("바깥")],
            blocks: vec![Block::Folding(Folding {
                summary: vec![text("안쪽")],
                blocks: vec![paragraph(vec![text("내용")])],
            })],
        })]
    );
}

#[test]
fn sized_inside_colored_inside_bold() {
    let document = parse("'''{{{#red {{{+2 강조}}}}}}'''");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Bold(vec![Inline::Colored(
            ColoredText {
                color: "red".to_string(),
                dark_color: None,
                content: vec![Inline::Sized(SizedText {
                    level: 2,
                    content: vec![text("강조")],
                })],
            }
        )])])]
    );
}

#[test]
fn footnote_inside_footnote() {
    let document = parse("본문[* 바깥 [* 안쪽]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            text("본문"),
            Inline::Footnote(Footnote {
                name: None,
                content: vec![
                    text("바깥 "),
                    Inline::Footnote(Footnote {
                        name: None,
                        content: vec![text("안쪽")],
                    }),
                ],
            }),
        ])]
    );
}

#[test]
fn triple_nested_unordered_lists() {
    let document = parse(" * 일\n  * 이\n   * 삼");
    assert_eq!(
        document.blocks,
        vec![unordered_list(vec![vec![
            paragraph(vec![text("일")]),
            unordered_list(vec![vec![
                paragraph(vec![text("이")]),
                unordered_list(vec![vec![paragraph(vec![text("삼")])]]),
            ]]),
        ]])]
    );
}

#[test]
fn ordered_list_with_nested_unordered_list() {
    let document = parse(" 1. 하나\n  * 점\n 1. 둘");
    assert_eq!(
        document.blocks,
        vec![Block::List(List {
            kind: ListKind::Decimal,
            items: vec![
                ListItem {
                    start_number: None,
                    blocks: vec![
                        paragraph(vec![text("하나")]),
                        unordered_list(vec![vec![paragraph(vec![text("점")])]]),
                    ],
                },
                ListItem {
                    start_number: None,
                    blocks: vec![paragraph(vec![text("둘")])],
                },
            ],
        })]
    );
}

#[test]
fn list_item_containing_code_block() {
    let document = parse(" * 항목\n  {{{\n  코드\n  }}}");
    assert_eq!(
        document.blocks,
        vec![unordered_list(vec![vec![
            paragraph(vec![text("항목")]),
            Block::CodeBlock(CodeBlock {
                language: None,
                source: "코드".to_string(),
            }),
        ]])]
    );
}

#[test]
fn table_inside_list_item() {
    let document = parse(" * 항목\n  || A ||");
    assert_eq!(
        document.blocks,
        vec![unordered_list(vec![vec![
            paragraph(vec![text("항목")]),
            Block::Table(Table {
                caption: None,
                rows: vec![TableRow {
                    cells: vec![simple_cell("A", HorizontalAlignment::Center)],
                }],
            }),
        ]])]
    );
}

#[test]
fn heading_content_with_markup() {
    let document = parse("== [[문서]] '''굵게''' ==");
    assert_eq!(
        document.blocks,
        vec![Block::Heading(Heading {
            level: 2,
            folded: false,
            content: vec![
                Inline::Link(Link {
                    anchor: None,
                    target: "문서".to_string(),
                    display: None,
                }),
                text(" "),
                Inline::Bold(vec![text("굵게")]),
            ],
        })]
    );
}

#[test]
fn link_display_containing_colored_text() {
    let document = parse("[[대상|{{{#blue 파랑}}}]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Link(Link {
            anchor: None,
            target: "대상".to_string(),
            display: Some(vec![Inline::Colored(ColoredText {
                color: "blue".to_string(),
                dark_color: None,
                content: vec![text("파랑")],
            })]),
        })])]
    );
}

#[test]
fn nested_quote_containing_list() {
    let document = parse("> 바깥\n>>  * 안쪽");
    assert_eq!(
        document.blocks,
        vec![Block::Quote(vec![
            paragraph(vec![text("바깥")]),
            Block::Quote(vec![unordered_list(vec![vec![paragraph(vec![text(
                "안쪽"
            )])]])]),
        ])]
    );
}
