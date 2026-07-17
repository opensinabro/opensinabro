use namumark_ast::{
    Block, CodeBlock, Footnote, Heading, Inline, Link, List, ListItem, ListKind, Macro,
};
use namumark_parser::parse;

fn text(content: &str) -> Inline {
    Inline::Text(content.to_string())
}

fn paragraph(inlines: Vec<Inline>) -> Block {
    Block::Paragraph(inlines)
}

#[test]
fn heading_levels() {
    let document = parse("= 개요 =\n====== 소단락 ======");
    assert_eq!(
        document.blocks,
        vec![
            Block::Heading(Heading {
                level: 1,
                folded: false,
                content: vec![text("개요")],
            }),
            Block::Heading(Heading {
                level: 6,
                folded: false,
                content: vec![text("소단락")],
            }),
        ]
    );
}

#[test]
fn folded_heading() {
    let document = parse("==# 접힌 문단 #==");
    assert_eq!(
        document.blocks,
        vec![Block::Heading(Heading {
            level: 2,
            folded: true,
            content: vec![text("접힌 문단")],
        })]
    );
}

#[test]
fn invalid_heading_is_paragraph() {
    let document = parse("=공백없음=");
    assert_eq!(document.blocks, vec![paragraph(vec![text("=공백없음=")])]);
}

#[test]
fn bold_and_nested_italic() {
    let document = parse("'''굵게 ''기울임'' 굵게'''");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Bold(vec![
            text("굵게 "),
            Inline::Italic(vec![text("기울임")]),
            text(" 굵게"),
        ])])]
    );
}

#[test]
fn strikethrough_markers() {
    let document = parse("~~물결~~ --대시--");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Strikethrough(vec![text("물결")]),
            text(" "),
            Inline::Strikethrough(vec![text("대시")]),
        ])]
    );
}

#[test]
fn underline_superscript_subscript() {
    let document = parse("__밑줄__ ^^위^^ ,,아래,,");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Underline(vec![text("밑줄")]),
            text(" "),
            Inline::Superscript(vec![text("위")]),
            text(" "),
            Inline::Subscript(vec![text("아래")]),
        ])]
    );
}

#[test]
fn backslash_escapes_markup() {
    let document = parse(r"\[\[링크 아님\]\]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![text("[[링크 아님]]")])]
    );
}

#[test]
fn inline_literal() {
    let document = parse("앞 {{{'''그대로'''}}} 뒤");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            text("앞 "),
            Inline::Literal("'''그대로'''".to_string()),
            text(" 뒤"),
        ])]
    );
}

#[test]
fn code_block_with_language() {
    let document = parse("{{{#!syntax rust\nfn main() {}\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::CodeBlock(CodeBlock {
            language: Some("rust".to_string()),
            source: "fn main() {}".to_string(),
        })])]
    );
}

#[test]
fn plain_multiline_literal_block() {
    let document = parse("{{{\n여러 줄\n그대로\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::CodeBlock(CodeBlock {
            language: None,
            source: "여러 줄\n그대로".to_string(),
        })])]
    );
}

#[test]
fn links() {
    let document = parse("[[대문]] [[대문|첫 화면]] [[https://example.com|예시]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Link(Link {
                anchor: None,
                target: "대문".into(),
                display: None,
            }),
            text(" "),
            Inline::Link(Link {
                anchor: None,
                target: "대문".into(),
                display: Some(vec![text("첫 화면")]),
            }),
            text(" "),
            Inline::Link(Link {
                anchor: None,
                target: "https://example.com".into(),
                display: Some(vec![text("예시")]),
            }),
        ])]
    );
}

#[test]
fn footnotes() {
    let document = parse("본문[* 각주 내용][*A 이름 있는 각주][*A]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            text("본문"),
            Inline::Footnote(Footnote {
                name: None,
                content: vec![text("각주 내용")],
            }),
            Inline::Footnote(Footnote {
                name: Some("A".to_string()),
                content: vec![text("이름 있는 각주")],
            }),
            Inline::Footnote(Footnote {
                name: Some("A".to_string()),
                content: vec![],
            }),
        ])]
    );
}

#[test]
fn footnote_containing_link() {
    let document = parse("본문[* [[문서]] 참고]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            text("본문"),
            Inline::Footnote(Footnote {
                name: None,
                content: vec![
                    Inline::Link(Link {
                        anchor: None,
                        target: "문서".into(),
                        display: None,
                    }),
                    text(" 참고"),
                ],
            }),
        ])]
    );
}

#[test]
fn macros() {
    let document = parse("[br] [age(2000-01-01)] [각주]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Macro(Macro {
                name: "br".to_string(),
                argument: None,
            }),
            text(" "),
            Inline::Macro(Macro {
                name: "age".to_string(),
                argument: Some("2000-01-01".into()),
            }),
            text(" "),
            Inline::Macro(Macro {
                name: "각주".to_string(),
                argument: None,
            }),
        ])]
    );
}

#[test]
fn nested_quote() {
    let document = parse(">인용\n>>중첩");
    assert_eq!(
        document.blocks,
        vec![Block::Quote(vec![
            paragraph(vec![text("인용")]),
            Block::Quote(vec![paragraph(vec![text("중첩")])]),
        ])]
    );
}

#[test]
fn horizontal_rule_comment_redirect() {
    let document = parse("#redirect 대문\n## 주석\n----");
    assert_eq!(
        document.blocks,
        vec![
            Block::Redirect("대문".into()),
            Block::Comment(" 주석".to_string()),
            Block::HorizontalRule,
        ]
    );
}

#[test]
fn unordered_list_with_nesting() {
    let document = parse(" * 항목1\n  * 하위\n * 항목2");
    assert_eq!(
        document.blocks,
        vec![Block::List(List {
            kind: ListKind::Unordered,
            items: vec![
                ListItem {
                    start_number: None,
                    blocks: vec![
                        paragraph(vec![text("항목1")]),
                        Block::List(List {
                            kind: ListKind::Unordered,
                            items: vec![ListItem {
                                start_number: None,
                                blocks: vec![paragraph(vec![text("하위")])],
                            }],
                        }),
                    ],
                },
                ListItem {
                    start_number: None,
                    blocks: vec![paragraph(vec![text("항목2")])],
                },
            ],
        })]
    );
}

#[test]
fn ordered_list_kinds_split() {
    let document = parse(" 1. 첫째\n a. 알파벳");
    assert_eq!(
        document.blocks,
        vec![
            Block::List(List {
                kind: ListKind::Decimal,
                items: vec![ListItem {
                    start_number: None,
                    blocks: vec![paragraph(vec![text("첫째")])],
                }],
            }),
            Block::List(List {
                kind: ListKind::LowerAlphabet,
                items: vec![ListItem {
                    start_number: None,
                    blocks: vec![paragraph(vec![text("알파벳")])],
                }],
            }),
        ]
    );
}

#[test]
fn indented_paragraph() {
    let document = parse(" 들여쓰기 문단");
    assert_eq!(
        document.blocks,
        vec![Block::Indent(vec![paragraph(vec![text("들여쓰기 문단")])])]
    );
}

// 빈 줄은 문단을 나누지 않고 줄바꿈 둘이 된다 — 나무위키의 문단은 헤딩 사이 구역이다.
#[test]
fn blank_line_is_two_line_breaks_within_one_paragraph() {
    let document = parse("첫 줄\n둘째 줄\n\n새 문단");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            text("첫 줄"),
            Inline::LineBreak,
            text("둘째 줄"),
            Inline::LineBreak,
            Inline::LineBreak,
            text("새 문단"),
        ])]
    );
}

// 블록 뒤 빈 줄의 개행은 뒤따르는 문단의 첫 줄바꿈이 된다
// (the seed: `</ul>` 뒤 빈 줄 → `<div class='wiki-paragraph'><br>…`).
#[test]
fn blank_line_after_block_leads_next_paragraph() {
    let document = parse(" * 항목\n\n문단");
    assert_eq!(
        document.blocks.last(),
        Some(&paragraph(vec![Inline::LineBreak, text("문단")]))
    );
}

// 문단 끝 빈 줄의 개행 하나는 문단 안에 남는다
// (the seed: `내용.\n\n== 다음 ==` → `<div class='wiki-paragraph'>내용.<br></div>`).
#[test]
fn blank_line_before_heading_keeps_one_line_break() {
    let document = parse("내용.\n\n== 다음 ==");
    assert_eq!(
        document.blocks.first(),
        Some(&paragraph(vec![text("내용."), Inline::LineBreak]))
    );
}

#[test]
fn unclosed_markup_is_plain_text() {
    let document = parse("'''닫히지 않음");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![text("'''닫히지 않음")])]
    );
}

use namumark_ast::{
    Category, ColoredText, Folding, HorizontalAlignment, Image, ImageOption, SizedText, Table,
    TableAttribute, TableAttributeScope, TableCell, TableRow, VerticalAlignment, WikiStyle,
};

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

#[test]
fn simple_table() {
    let document = parse("|| A || B ||\n|| C || D ||");
    assert_eq!(
        document.blocks,
        vec![Block::Table(Table {
            caption: None,
            rows: vec![
                TableRow {
                    cells: vec![
                        simple_cell("A", HorizontalAlignment::Center),
                        simple_cell("B", HorizontalAlignment::Center),
                    ],
                },
                TableRow {
                    cells: vec![
                        simple_cell("C", HorizontalAlignment::Center),
                        simple_cell("D", HorizontalAlignment::Center),
                    ],
                },
            ],
        })]
    );
}

#[test]
fn table_alignment_by_spaces() {
    let document = parse("||왼쪽 || 오른쪽|| 가운데 ||");
    assert_eq!(
        document.blocks,
        vec![Block::Table(Table {
            caption: None,
            rows: vec![TableRow {
                cells: vec![
                    simple_cell("왼쪽", HorizontalAlignment::Left),
                    simple_cell("오른쪽", HorizontalAlignment::Right),
                    simple_cell("가운데", HorizontalAlignment::Center),
                ],
            }],
        })]
    );
}

#[test]
fn table_caption() {
    let document = parse("|캡션| A ||");
    assert_eq!(
        document.blocks,
        vec![Block::Table(Table {
            caption: Some(vec![text("캡션")]),
            rows: vec![TableRow {
                cells: vec![simple_cell("A", HorizontalAlignment::Center)],
            }],
        })]
    );
}

#[test]
fn table_automatic_column_span() {
    let document = parse("|||| 병합 ||");
    let Block::Table(table) = &document.blocks[0] else {
        panic!("expected table");
    };
    assert_eq!(table.rows[0].cells[0].column_span, Some(2));
}

#[test]
fn table_cell_options() {
    let document = parse("||<-3><^|2><:><bgcolor=#eee><table align=center>내용||");
    let Block::Table(table) = &document.blocks[0] else {
        panic!("expected table");
    };
    let cell = &table.rows[0].cells[0];
    assert_eq!(cell.column_span, Some(3));
    assert_eq!(cell.row_span, Some(2));
    assert_eq!(cell.horizontal_alignment, Some(HorizontalAlignment::Center));
    assert_eq!(cell.vertical_alignment, Some(VerticalAlignment::Top));
    assert_eq!(
        cell.attributes,
        vec![
            TableAttribute {
                scope: TableAttributeScope::Cell,
                name: "bgcolor".to_string(),
                value: Some("#eee".into()),
            },
            TableAttribute {
                scope: TableAttributeScope::Table,
                name: "align".to_string(),
                value: Some("center".into()),
            },
        ]
    );
    assert_eq!(cell.blocks, vec![paragraph(vec![text("내용")])]);
}

#[test]
fn table_bare_color_option() {
    let document = parse("||<#ddd> 회색 ||");
    let Block::Table(table) = &document.blocks[0] else {
        panic!("expected table");
    };
    let cell = &table.rows[0].cells[0];
    assert_eq!(
        cell.attributes,
        vec![TableAttribute {
            scope: TableAttributeScope::Cell,
            name: "bgcolor".to_string(),
            value: Some("#ddd".into()),
        }]
    );
    assert_eq!(cell.horizontal_alignment, Some(HorizontalAlignment::Center));
}

#[test]
fn table_multiline_cell() {
    let document = parse("|| 첫 줄\n둘째 줄 ||");
    let Block::Table(table) = &document.blocks[0] else {
        panic!("expected table");
    };
    let cell = &table.rows[0].cells[0];
    assert_eq!(cell.horizontal_alignment, Some(HorizontalAlignment::Center));
    assert_eq!(
        cell.blocks,
        vec![paragraph(vec![
            text("첫 줄"),
            Inline::LineBreak,
            text("둘째 줄"),
        ])]
    );
}

#[test]
fn wiki_style_block() {
    let document =
        parse("{{{#!wiki style=\"margin: 10px\" dark-style='color: white'\n'''내용'''\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::WikiStyle(WikiStyle {
            style: Some("margin: 10px".into()),
            dark_style: Some("color: white".into()),
            blocks: vec![paragraph(vec![Inline::Bold(vec![text("내용")])])],
        })])]
    );
}

#[test]
fn folding_block() {
    let document = parse("{{{#!folding 펼치기\n숨은 내용\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Folding(Folding {
            summary: "펼치기".into(),
            blocks: vec![paragraph(vec![text("숨은 내용")])],
        })])]
    );
}

#[test]
fn html_block() {
    let document = parse("{{{#!html\n<b>굵게</b>\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Html("<b>굵게</b>".into())])]
    );
}

#[test]
fn inline_colored_text() {
    let document = parse("{{{#red 빨강}}} {{{#ff0000,#00ff00 듀얼}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Colored(ColoredText {
                color: "red".to_string(),
                dark_color: None,
                content: vec![text("빨강")],
            }),
            text(" "),
            Inline::Colored(ColoredText {
                color: "#ff0000".to_string(),
                dark_color: Some("#00ff00".to_string()),
                content: vec![text("듀얼")],
            }),
        ])]
    );
}

#[test]
fn inline_sized_text() {
    let document = parse("{{{+3 크게}}} {{{-2 작게}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Sized(SizedText {
                level: 3,
                content: vec![text("크게")],
            }),
            text(" "),
            Inline::Sized(SizedText {
                level: -2,
                content: vec![text("작게")],
            }),
        ])]
    );
}

#[test]
fn invalid_color_stays_literal() {
    let document = parse("{{{#a-b 텍스트}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Literal("#a-b 텍스트".to_string())])]
    );
}

#[test]
fn multiline_sized_block() {
    let document = parse("{{{+1\n첫 줄\n둘째 줄\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Sized(SizedText {
            level: 1,
            content: vec![text("첫 줄"), Inline::LineBreak, text("둘째 줄")],
        })])]
    );
}

// 색상·크기는 서식일 뿐이라 안쪽 내용을 인라인으로 편다.
// 그 안에 표를 넣는 형태는 나무위키 렌더에서도 나타나지 않는다.
#[test]
fn multiline_colored_block_flattens_to_inline() {
    let document = parse("{{{#red\n빨강\n}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Colored(ColoredText {
            color: "red".to_string(),
            dark_color: None,
            content: vec![text("빨강")],
        })])]
    );
}

// 나무위키에서 `{{{` 그룹은 인라인 요소다 — 문단 중간에서 열려도 문단을 쪼개지 않는다.
#[test]
fn brace_group_opened_in_paragraph_middle() {
    let document = parse("앞 텍스트 {{{#!wiki\n|| A ||\n}}} 뒤 텍스트");
    assert_eq!(document.blocks.len(), 1, "{:?}", document.blocks);
    let Block::Paragraph(inlines) = &document.blocks[0] else {
        panic!("문단이어야 한다");
    };
    assert_eq!(inlines[0], text("앞 텍스트 "));
    let Inline::WikiStyle(wiki_style) = &inlines[1] else {
        panic!("wiki 그룹이어야 한다: {inlines:?}");
    };
    assert!(matches!(wiki_style.blocks[0], Block::Table(_)));
    assert_eq!(inlines[2], text(" 뒤 텍스트"));
}

#[test]
fn nested_link_in_display() {
    let document = parse("[[문서|[[파일:아이콘.png|width=20]]]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Link(Link {
            target: "문서".into(),
            anchor: None,
            display: Some(vec![Inline::Image(Image {
                file_name: "아이콘.png".into(),
                options: vec![ImageOption {
                    name: "width".to_string(),
                    value: Some("20".into()),
                }],
            })]),
        })])]
    );
}

#[test]
fn image_link_with_options() {
    let document = parse("[[파일:예시.png|width=100%&align=center]] [[file:x.png]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![
            Inline::Image(Image {
                file_name: "예시.png".into(),
                options: vec![
                    ImageOption {
                        name: "width".to_string(),
                        value: Some("100%".into()),
                    },
                    ImageOption {
                        name: "align".to_string(),
                        value: Some("center".into()),
                    },
                ],
            }),
            text(" "),
            Inline::Image(Image {
                file_name: "x.png".into(),
                options: vec![],
            }),
        ])]
    );
}

#[test]
fn category_link() {
    let document = parse("[[분류:음악]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Category(Category {
            name: "음악".to_string(),
        })])]
    );
}

#[test]
fn link_anchor_is_split() {
    let document = parse("[[1993년 한국시리즈#s-5.2|5차전]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Link(Link {
            target: "1993년 한국시리즈".into(),
            anchor: Some("s-5.2".into()),
            display: Some(vec![text("5차전")]),
        })])]
    );
}

#[test]
fn external_url_keeps_fragment() {
    let document = parse("[[https://example.com/a#frag]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Link(Link {
            target: "https://example.com/a#frag".into(),
            anchor: None,
            display: None,
        })])]
    );
}

#[test]
fn colon_escaped_file_link_is_plain_link() {
    let document = parse("[[:파일:포스터.jpg]]");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Link(Link {
            target: "파일:포스터.jpg".into(),
            anchor: None,
            display: None,
        })])]
    );
}

#[test]
fn ordered_list_start_number() {
    let document = parse(" 1.#42 항목");
    assert_eq!(
        document.blocks,
        vec![Block::List(List {
            kind: ListKind::Decimal,
            items: vec![ListItem {
                start_number: Some(42),
                blocks: vec![paragraph(vec![text("항목")])],
            }],
        })]
    );
}

#[test]
fn list_marker_without_space() {
    let document = parse(" *항목");
    assert_eq!(
        document.blocks,
        vec![Block::List(List {
            kind: ListKind::Unordered,
            items: vec![ListItem {
                start_number: None,
                blocks: vec![paragraph(vec![text("항목")])],
            }],
        })]
    );
}

// 순서 리스트 마커는 항상 `1.` 리터럴이라 `2.`는 마커가 아니다. 마커가 아닌 줄은
// 앞 항목의 문단이 이어지는 것이다.
#[test]
fn literal_number_is_not_list_marker() {
    let document = parse(" 1. 하나\n 2. 둘");
    assert_eq!(
        document.blocks,
        vec![Block::List(List {
            kind: ListKind::Decimal,
            items: vec![ListItem {
                start_number: None,
                blocks: vec![paragraph(vec![
                    text("하나"),
                    Inline::LineBreak,
                    text("2. 둘"),
                ])],
            }],
        })]
    );
}

// 색상 표기 뒤에 내용을 가르는 공백이 없으면 색상이 아니다.
// 렌더확정: the seed는 `{{{#212529}}}`를 `<code>#212529</code>`로 낸다.
#[test]
fn color_group_without_separator_is_a_literal() {
    let document = parse("{{{#212529}}}");
    assert_eq!(
        document.blocks,
        vec![paragraph(vec![Inline::Literal("#212529".to_string())])]
    );
}

// 인용 마커는 `>` 하나뿐이다. 뒤따르는 공백은 마커가 아니라 들여쓰기 한 단계다
// (렌더확정: `> 내용`은 `<blockquote><div class='wiki-indent'>…`, `>내용`은 들여쓰기가 없다).
#[test]
fn space_after_quote_marker_is_an_indent() {
    assert_eq!(
        parse(">인용문").blocks,
        vec![Block::Quote(vec![paragraph(vec![text("인용문")])])]
    );
    assert_eq!(
        parse("> 인용문").blocks,
        vec![Block::Quote(vec![Block::Indent(vec![paragraph(vec![
            text("인용문")
        ])])])]
    );
}

// 주석 줄은 개행까지 통째로 사라진다 — 문단을 끊지도, 줄바꿈을 남기지도 않는다
// (렌더확정: the seed는 `가나다라\n## 주석\n마바사아`를 `가나다라<br>마바사아`로 낸다).
#[test]
fn comment_line_leaves_no_line_break() {
    assert_eq!(
        parse("가나다라\n## 주석\n마바사아").blocks,
        vec![paragraph(vec![
            text("가나다라"),
            Inline::LineBreak,
            text("마바사아"),
        ])]
    );
}

// 접기 문구에는 위키 문법이 적용되지 않는다 — 글자 그대로다
// (렌더확정: the seed는 `{{{#!folding '''[ 펼치기 ]'''`의 `'''`를 풀지 않고 보여 준다).
#[test]
fn folding_summary_is_plain_text() {
    let document = parse("{{{#!folding '''[ 펼치기 ]'''\n내용\n}}}");
    let [Block::Paragraph(inlines)] = document.blocks.as_slice() else {
        panic!("문단이어야 한다: {:?}", document.blocks);
    };
    let [Inline::Folding(folding)] = inlines.as_slice() else {
        panic!("접기여야 한다: {inlines:?}");
    };
    assert_eq!(folding.summary.to_string(), "'''[ 펼치기 ]'''");
}
