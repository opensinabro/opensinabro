mod model;

use model::{
    Block, CodeBlock, ColoredText, Folding, Footnote, Heading, Inline, Link, List, ListItem,
    SizedText, Table, TableCell, TableRow, WikiStyle,
};
use namumark_ast::{HorizontalAlignment, ListKind};
use namumark_parser::parse;

fn text(content: &str) -> Inline {
    Inline::Text(content.to_string())
}

fn paragraph(inlines: Vec<Inline>) -> Block {
    Block::Paragraph(inlines)
}

fn simple_cell(content: &str, alignment: HorizontalAlignment) -> TableCell {
    TableCell {
        column_span: None,
        row_span: None,
        horizontal_alignment: Some(alignment),
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
    let document = parse(">|| A ||\n> * 항목\n>텍스트");
    assert_eq!(
        model::of(&document),
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
        model::of(&document),
        vec![Block::Table(Table {
            caption: None,
            rows: vec![TableRow {
                cells: vec![TableCell {
                    column_span: None,
                    row_span: None,
                    horizontal_alignment: Some(HorizontalAlignment::Center),
                    vertical_alignment: None,
                    attributes: vec![],
                    blocks: vec![paragraph(vec![
                        text("첫"),
                        Inline::LineBreak,
                        Inline::Folding(Folding {
                            summary: "보기".into(),
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
                        Inline::LineBreak,
                        text("끝"),
                    ])],
                }],
            }],
        })]
    );
}

#[test]
fn wiki_style_containing_list_and_table() {
    let document = parse("{{{#!wiki\n * 항목\n|| A ||\n}}}");
    assert_eq!(
        model::of(&document),
        vec![paragraph(vec![Inline::WikiStyle(WikiStyle {
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
        })])]
    );
}

#[test]
fn nested_folding_blocks() {
    let document = parse("{{{#!folding 바깥\n{{{#!folding 안쪽\n내용\n}}}\n}}}");
    assert_eq!(
        model::of(&document),
        vec![paragraph(vec![Inline::Folding(Folding {
            summary: "바깥".into(),
            blocks: vec![paragraph(vec![Inline::Folding(Folding {
                summary: "안쪽".into(),
                blocks: vec![paragraph(vec![text("내용")])],
            })])],
        })])]
    );
}

#[test]
fn sized_inside_colored_inside_bold() {
    let document = parse("'''{{{#red {{{+2 강조}}}}}}'''");
    assert_eq!(
        model::of(&document),
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
        model::of(&document),
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
        model::of(&document),
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
        model::of(&document),
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
        model::of(&document),
        vec![unordered_list(vec![vec![
            paragraph(vec![text("항목")]),
            Block::Indent(vec![paragraph(vec![Inline::CodeBlock(CodeBlock {
                language: None,
                source: "코드".to_string(),
            })])]),
        ]])]
    );
}

// 항목보다 한 칸 더 들어간 속내용은 들여쓰기 한 단계다(리스트가 아니므로).
#[test]
fn table_inside_list_item() {
    let document = parse(" * 항목\n  || A ||");
    assert_eq!(
        model::of(&document),
        vec![unordered_list(vec![vec![
            paragraph(vec![text("항목")]),
            Block::Indent(vec![Block::Table(Table {
                caption: None,
                rows: vec![TableRow {
                    cells: vec![simple_cell("A", HorizontalAlignment::Center)],
                }],
            })]),
        ]])]
    );
}

#[test]
fn heading_content_with_markup() {
    let document = parse("== [[문서]] '''굵게''' ==");
    assert_eq!(
        model::of(&document),
        vec![Block::Heading(Heading {
            level: 2,
            folded: false,
            content: vec![
                Inline::Link(Link {
                    anchor: None,
                    target: "문서".into(),
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
        model::of(&document),
        vec![paragraph(vec![Inline::Link(Link {
            anchor: None,
            target: "대상".into(),
            display: Some(vec![Inline::Colored(ColoredText {
                color: "blue".to_string(),
                dark_color: None,
                content: vec![text("파랑")],
            })]),
        })])]
    );
}

// 인용 마커는 `>` 하나뿐이고 뒤따르는 공백은 들여쓰기 한 단계다.
#[test]
fn nested_quote_containing_list() {
    let document = parse("> 바깥\n>>  * 안쪽");
    assert_eq!(
        model::of(&document),
        vec![Block::Quote(vec![
            Block::Indent(vec![paragraph(vec![text("바깥")])]),
            Block::Quote(vec![Block::Indent(vec![unordered_list(vec![vec![
                paragraph(vec![text("안쪽")])
            ]])])]),
        ])]
    );
}

// 줄머리 문맥(인용·들여쓰기·리스트) 안에서 열린 여러 줄 `{{{` 그룹은
// 마커가 없는 뒷줄까지 이어진다. 나무위키 문법 도움말의 인용문·리스트 예제가 이 형태다.

#[test]
fn quote_containing_multiline_wiki_style() {
    let document = parse(">{{{#!wiki style=\"margin:1em\"\n인용문}}}");
    assert_eq!(
        model::of(&document),
        vec![Block::Quote(vec![paragraph(vec![Inline::WikiStyle(
            WikiStyle {
                style: Some("margin:1em".into()),
                dark_style: None,
                blocks: vec![paragraph(vec![text("인용문")])],
            }
        )])])]
    );
}

#[test]
fn list_item_containing_multiline_wiki_style() {
    let document = parse(" * {{{#!wiki style=\"display: inline\"\n내용}}}");
    assert_eq!(
        model::of(&document),
        vec![unordered_list(vec![vec![paragraph(vec![
            Inline::WikiStyle(WikiStyle {
                style: Some("display: inline".into()),
                dark_style: None,
                blocks: vec![paragraph(vec![text("내용")])],
            })
        ])]])]
    );
}

#[test]
fn list_item_containing_multiline_folding() {
    let document = parse(" * {{{#!folding [ 펼치기 ]\n내용\n}}}");
    assert_eq!(
        model::of(&document),
        vec![unordered_list(vec![vec![paragraph(vec![
            Inline::Folding(Folding {
                summary: "[ 펼치기 ]".into(),
                blocks: vec![paragraph(vec![text("내용")])],
            })
        ])]])]
    );
}

#[test]
fn indent_containing_multiline_wiki_style() {
    let document = parse(" 들여쓰기 {{{#!wiki style=\"margin:0\"\n내용}}}");
    assert_eq!(
        model::of(&document),
        vec![Block::Indent(vec![paragraph(vec![
            text("들여쓰기 "),
            Inline::WikiStyle(WikiStyle {
                style: Some("margin:0".into()),
                dark_style: None,
                blocks: vec![paragraph(vec![text("내용")])],
            }),
        ])])]
    );
}

// 닫히지 않은 그룹은 줄머리 마커 규칙만으로 수집한다(뒷줄을 삼키지 않는다).
#[test]
fn quote_with_unclosed_group_keeps_marker_rule() {
    let document = parse(">{{{#!wiki style=\"margin:1em\"\n바깥 문단");
    assert_eq!(
        model::of(&document),
        vec![
            Block::Quote(vec![paragraph(vec![text(
                "{{{#!wiki style=\"margin:1em\""
            )])]),
            paragraph(vec![text("바깥 문단")]),
        ]
    );
}

// 닫히지 않는 `{{{`는 그룹이 아니라 글자다 — 셀 구분자 `||`를 가리지 않는다.
// 렌더확정: `||<-3> {{{{{{-5 {{{-5 -10단계}}}}}} ||<-3> …||`를 the seed는 두 셀로 내고,
// 첫 셀은 `{` + 리터럴 `{{-5 {{{-5 -10단계}}}`가 된다.
#[test]
fn unclosed_group_does_not_swallow_cell_separator() {
    let document = parse("||<-3> {{{{{{-5 {{{-5 -10단계}}}}}} ||<-3> 뒤 ||");
    let blocks = model::of(&document);
    let [Block::Table(table)] = blocks.as_slice() else {
        panic!("표 하나여야 한다: {blocks:?}");
    };
    let [row] = table.rows.as_slice() else {
        panic!("행 하나여야 한다");
    };
    assert_eq!(row.cells.len(), 2, "{:?}", row.cells);
    assert_eq!(
        row.cells[0].blocks,
        vec![paragraph(vec![
            text("{"),
            Inline::Literal("{{-5 {{{-5 -10단계}}}".to_string()),
        ])]
    );
}

// 항목 줄 다음, 마커도 들여쓰기도 없는 줄은 그 항목의 문단이 이어지는 것이다.
// 렌더확정: the seed는 ` * {{{…}}}\n #설명`을
// `<li><div class='wiki-paragraph'><code>…</code><br>#설명</div></li>`로 낸다.
#[test]
fn list_item_paragraph_continues_on_unmarked_line() {
    let document = parse(" * 항목\n 이어짐");
    assert_eq!(
        model::of(&document),
        vec![unordered_list(vec![vec![paragraph(vec![
            text("항목"),
            Inline::LineBreak,
            text("이어짐"),
        ])]])]
    );
}
