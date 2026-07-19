use namumark_ir::{DocumentLinkKind, RenderBlock, RenderInline, RenderTree, TextStyle};
use namumark_render::{
    Date, DateTime, EmptyContext, Time, WikiContext, build_render_tree,
    build_render_tree_with_diagnostics,
};
use std::collections::HashMap;

struct TestContext {
    documents: HashMap<String, String>,
    now: Option<DateTime>,
    title: Option<String>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
            now: None,
            title: None,
        }
    }
}

impl WikiContext for TestContext {
    fn document_exists(&self, title: &str) -> bool {
        self.documents.contains_key(title)
    }

    fn current_title(&self) -> Option<String> {
        self.title.clone()
    }

    fn include_source(&self, title: &str) -> Option<String> {
        self.documents.get(title).cloned()
    }

    fn now(&self) -> Option<DateTime> {
        self.now
    }
}

fn tree(source: &str) -> RenderTree {
    build_render_tree(&namumark_parser::parse(source), &EmptyContext)
}

fn diagnostic_codes(source: &str) -> Vec<String> {
    let (_, diagnostics) =
        build_render_tree_with_diagnostics(&namumark_parser::parse(source), &EmptyContext);
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.to_string())
        .collect()
}

#[test]
fn unknown_macro_is_reported() {
    assert_eq!(
        diagnostic_codes("[아무개매크로]"),
        vec!["unsupported-macro"]
    );
}

#[test]
fn known_macro_is_not_reported() {
    assert!(diagnostic_codes("[목차]").is_empty());
}

#[test]
fn known_macro_without_required_argument_is_reported() {
    assert_eq!(diagnostic_codes("[anchor]"), vec!["invalid-macro-argument"]);
    assert_eq!(diagnostic_codes("[math]"), vec!["invalid-macro-argument"]);
    assert_eq!(
        diagnostic_codes("[youtube()]"),
        vec!["invalid-macro-argument"]
    );
}

#[test]
fn bad_date_argument_is_reported() {
    assert_eq!(
        diagnostic_codes("[age(날짜아님)]"),
        vec!["invalid-macro-argument"]
    );
}

/// now()가 없어 원문 표기로 남는 것은 렌더 결정성 정책이지 저자 잘못이 아니다.
#[test]
fn date_macro_without_context_now_is_not_reported() {
    assert!(diagnostic_codes("[date]").is_empty());
    assert!(diagnostic_codes("[age(1990-01-01)]").is_empty());
}

#[test]
fn missing_include_target_is_reported() {
    assert_eq!(
        diagnostic_codes("[include(틀:없음)]"),
        vec!["include-target-missing"]
    );
}

#[test]
fn existing_include_target_is_not_reported() {
    let mut context = TestContext::new();
    context
        .documents
        .insert("틀:있음".to_string(), "내용".to_string());
    let (_, diagnostics) =
        build_render_tree_with_diagnostics(&namumark_parser::parse("[include(틀:있음)]"), &context);
    assert!(diagnostics.is_empty());
}

#[test]
fn redirect_to_self_is_reported() {
    let mut context = TestContext::new();
    context.title = Some("대문".to_string());
    let (_, diagnostics) =
        build_render_tree_with_diagnostics(&namumark_parser::parse("#redirect 대문"), &context);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect::<Vec<_>>(),
        vec!["self-redirect"]
    );
}

#[test]
fn redirect_to_other_document_is_not_reported() {
    let mut context = TestContext::new();
    context.title = Some("다른 문서".to_string());
    let (_, diagnostics) =
        build_render_tree_with_diagnostics(&namumark_parser::parse("#redirect 대문"), &context);
    assert!(diagnostics.is_empty());
}

#[test]
fn macro_inside_included_template_is_not_reported() {
    let mut context = TestContext::new();
    context
        .documents
        .insert("틀:샘플".to_string(), "[안에서쓴미지원매크로]".to_string());
    let (_, diagnostics) =
        build_render_tree_with_diagnostics(&namumark_parser::parse("[include(틀:샘플)]"), &context);
    assert!(diagnostics.is_empty());
}

#[test]
fn heading_numbers_are_hierarchical() {
    let tree = tree("== 하나 ==\n=== 하나둘 ===\n== 둘 ==");
    let numbers: Vec<String> = tree
        .blocks
        .iter()
        .filter_map(|block| match block {
            RenderBlock::Heading { number, .. } => Some(number.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, vec!["1", "1.1", "2"]);
}

/// 목차는 트리 밖 최상위 목록이 소유하므로 `[목차]`가 어디에 놓이든 같은 것을 그린다.
///
/// 예전에는 layout이 트리를 다시 훑어 `[목차]` 자리를 채웠는데, 그 순회가 본 순회의
/// 축소 복제본이라 서식·표 캡션 안으로 내려가지 않아 그 자리의 목차가 빈 채로 남았다.
#[test]
fn table_of_contents_is_document_wide_wherever_it_sits() {
    let tree = tree("'''[목차]'''\n||[목차]||\n== 하나 ==\n=== 하나둘 ===");

    let numbers: Vec<&str> = tree
        .table_of_contents
        .iter()
        .map(|entry| entry.number.as_str())
        .collect();
    assert_eq!(numbers, vec!["1", "1.1"]);

    // 자리표시는 서식 안에도, 표 셀 안에도 그대로 있다.
    let mut placeholders = 0;
    walk_tree(&tree, &mut |inline| {
        if matches!(inline, RenderInline::TableOfContents) {
            placeholders += 1;
        }
    });
    assert_eq!(placeholders, 2, "서식·표 안의 [목차] 자리가 사라졌다");
}

#[test]
fn footnotes_are_numbered_and_merged() {
    let tree = tree("본문[* 첫째][*A 이름 각주][*A] 끝");
    // 문서 끝에 잔여 각주 섹션이 자동 방출된다. `[각주]`는 매크로라 문단 안에 놓인다.
    let Some(RenderBlock::Paragraph { content: inlines }) = tree.blocks.last() else {
        panic!("문서 끝 각주 문단이 있어야 한다");
    };
    let Some(RenderInline::FootnoteSection { notes }) = inlines.first() else {
        panic!("각주 섹션이 있어야 한다: {inlines:?}");
    };
    // 섹션은 인덱스만 들고, 내용은 트리 최상위 각주 목록이 소유한다.
    assert_eq!(notes, &[0, 1]);
    assert_eq!(tree.footnotes.len(), 2);
    assert_eq!(tree.footnotes[0].label, "1");
    assert_eq!(tree.footnotes[1].label, "A");
    assert_eq!(tree.footnotes[1].reference_numbers.len(), 2);
}

#[test]
fn footnote_macro_flushes_pending_notes() {
    let tree = tree("본문[* 하나]\n[각주]\n다음[* 둘]");
    let section_labels: Vec<Vec<String>> = tree
        .blocks
        .iter()
        .flat_map(|block| match block {
            RenderBlock::Paragraph { content: inlines } => inlines.as_slice(),
            _ => &[],
        })
        .filter_map(|inline| match inline {
            RenderInline::FootnoteSection { notes } => Some(
                notes
                    .iter()
                    .map(|index| tree.footnotes[*index as usize].label.clone())
                    .collect(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        section_labels,
        vec![vec!["1".to_string()], vec!["2".to_string()]]
    );
}

/// 같은 이름 각주라도 `[각주]`로 끊기면 **다른 각주**다 — 병합은 미방출 구간 안에서만
/// 일어나기 때문이다. 그래서 라벨은 문서 전체에서 유일하지 않고, 각주를 가리키는 키로
/// 쓸 수 있는 것은 참조 번호뿐이다(프론트엔드의 미리보기 사전과 복귀 앵커가 이걸 딛는다).
#[test]
fn same_name_across_sections_makes_separate_footnotes() {
    let tree = tree("본문[*A 첫째]\n[각주]\n다음[*A 둘째]\n[각주]");

    let labels: Vec<&str> = tree
        .footnotes
        .iter()
        .map(|footnote| footnote.label.as_str())
        .collect();
    assert_eq!(labels, vec!["A", "A"], "라벨은 겹친다");

    let references: Vec<&[u32]> = tree
        .footnotes
        .iter()
        .map(|footnote| footnote.reference_numbers.as_slice())
        .collect();
    assert_eq!(
        references,
        vec![&[1u32][..], &[2u32][..]],
        "번호는 안 겹친다"
    );
}

#[test]
fn include_is_expanded_with_arguments() {
    let mut context = TestContext::new();
    context.documents.insert(
        "틀:인사".to_string(),
        "'''@이름@''' 님, 안녕하세요.".to_string(),
    );
    let document = namumark_parser::parse("[include(틀:인사, 이름=단풍)]");
    let tree = build_render_tree(&document, &context);
    let RenderBlock::Paragraph { content: inlines } = &tree.blocks[0] else {
        panic!("문단이어야 한다");
    };
    assert_eq!(
        inlines[0],
        RenderInline::Styled {
            style: TextStyle::Bold,
            content: vec![RenderInline::Text {
                text: "단풍".to_string()
            }],
        }
    );
}

// 렌더확정: 원문 `[[분류:X]]\n[include(틀:Y)]\n[목차]\n[clearfix]`를 the seed는
// `<div class='wiki-paragraph'><목차><br><clearfix></div>`로 낸다 — 분류·include만 있는
// 줄은 개행까지 사라지고, `[목차]` 줄의 개행만 `<br>`이 된다.
#[test]
fn category_and_include_lines_leave_no_line_break() {
    let mut context = TestContext::new();
    context
        .documents
        .insert("틀:안내".to_string(), "안내".to_string());
    let document =
        namumark_parser::parse("[[분류:도움말]]\n[include(틀:안내)]\n[목차]\n[clearfix]");
    let tree = build_render_tree(&document, &context);
    let RenderBlock::Paragraph { content: inlines } = tree.blocks.last().unwrap() else {
        panic!("문단이어야 한다: {:?}", tree.blocks);
    };
    assert!(
        matches!(inlines.as_slice(), [_, RenderInline::LineBreak, _]),
        "{inlines:?}"
    );
}

#[test]
fn nested_include_is_not_expanded() {
    // 나무위키는 틀 속의 틀을 확장하지 않는다. 원문을 노출하지도 않고 조용히 버린다.
    // 이 규칙 덕에 순환 include도 구조적으로 성립하지 않는다.
    let mut context = TestContext::new();
    context
        .documents
        .insert("틀:루프".to_string(), "[include(틀:루프)]".to_string());
    let document = namumark_parser::parse("[include(틀:루프)]");
    let tree = build_render_tree(&document, &context);
    assert!(tree.blocks.is_empty(), "{:?}", tree.blocks);
}

#[test]
fn included_template_drops_its_own_include() {
    let mut context = TestContext::new();
    context.documents.insert(
        "틀:안내".to_string(),
        "안내 본문\n[include(틀:안내/설명 문서)]".to_string(),
    );
    context.documents.insert(
        "틀:안내/설명 문서".to_string(),
        "== 사용법 ==\n설명".to_string(),
    );
    let document = namumark_parser::parse("[include(틀:안내)]");
    let tree = build_render_tree(&document, &context);
    // 틀 본문은 들어오지만, 그 틀이 품은 설명 문서는 딸려오지 않는다.
    let rendered = format!("{:?}", tree.blocks);
    assert!(rendered.contains("안내 본문"), "{rendered}");
    assert!(!rendered.contains("사용법"), "{rendered}");
}

#[test]
fn link_existence_is_resolved() {
    let mut context = TestContext::new();
    context.documents.insert("존재".to_string(), String::new());
    let document = namumark_parser::parse("[[존재]] [[없음]]");
    let tree = build_render_tree(&document, &context);
    let RenderBlock::Paragraph { content: inlines } = &tree.blocks[0] else {
        panic!("문단이어야 한다");
    };
    assert!(matches!(
        &inlines[0],
        RenderInline::DocumentLink {
            kind: DocumentLinkKind::Existing,
            ..
        }
    ));
    assert!(matches!(
        &inlines[2],
        RenderInline::DocumentLink {
            kind: DocumentLinkKind::Missing,
            ..
        }
    ));
}

#[test]
fn age_uses_context_today() {
    let mut context = TestContext::new();
    context.now = Some(DateTime {
        date: Date {
            year: 2026,
            month: 7,
            day: 17,
        },
        time: Time {
            hour: 12,
            minute: 34,
            second: 56,
        },
    });
    let document = namumark_parser::parse("[age(2000-01-01)]");
    let tree = build_render_tree(&document, &context);
    let RenderBlock::Paragraph { content: inlines } = &tree.blocks[0] else {
        panic!("문단이어야 한다");
    };
    assert_eq!(
        inlines[0],
        RenderInline::Text {
            text: "26".to_string()
        }
    );
}

#[test]
fn age_without_today_stays_unresolved() {
    let tree = tree("[age(2000-01-01)]");
    assert!(format!("{:?}", tree.blocks).contains("Unresolved"));
}

#[test]
fn categories_are_collected_and_removed() {
    let tree = tree("[[분류:음악]][[분류:역사]]\n본문");
    assert_eq!(tree.categories, vec!["음악", "역사"]);
    assert!(!format!("{:?}", tree.blocks).contains("분류"));
}

#[test]
fn redirect_is_lifted_to_document() {
    let tree = tree("#redirect 대문");
    assert_eq!(tree.redirect.as_deref(), Some("대문"));
}

#[test]
fn date_macro_uses_context_now() {
    let mut context = TestContext::new();
    context.now = Some(DateTime {
        date: Date {
            year: 2026,
            month: 7,
            day: 17,
        },
        time: Time {
            hour: 9,
            minute: 5,
            second: 3,
        },
    });
    let document = namumark_parser::parse("[date] / [datetime]");
    let tree = build_render_tree(&document, &context);
    let text = format!("{:?}", tree.blocks);
    assert!(text.contains("2026-07-17 09:05:03"));
}

#[test]
fn dimension_and_color_are_specialized() {
    use namumark_ir::{ColorValue, Dimension};
    assert_eq!(Dimension::parse("550"), Dimension::Pixels(550));
    assert_eq!(Dimension::parse("100%"), Dimension::Percentage(100));
    assert_eq!(Dimension::parse("20px"), Dimension::Pixels(20));
    assert_eq!(
        Dimension::parse("1.5em"),
        Dimension::Custom("1.5em".to_string())
    );
    assert_eq!(
        ColorValue::parse("#fff"),
        Some(ColorValue::Rgb {
            red: 255,
            green: 255,
            blue: 255,
        })
    );
    assert_eq!(
        ColorValue::parse("red"),
        Some(ColorValue::Named("red".to_string()))
    );
    assert_eq!(
        ColorValue::parse("#ff8000").map(|color| color.to_string()),
        Some("#ff8000".to_string())
    );
    assert_eq!(Dimension::Percentage(80).to_string(), "80%");
}

// 나무위키는 색 자리에 아무 문자열이나 받지 않는다. 틀 인자가 안 채워져 `#배경색`
// 같은 값이 들어오면 그 선언을 통째로 버린다.
#[test]
fn color_rejects_what_is_not_a_color() {
    use namumark_ir::ColorValue;
    assert_eq!(ColorValue::parse("#배경색"), None);
    assert_eq!(ColorValue::parse("br"), None);
    assert_eq!(ColorValue::parse("#ff"), None);
    // CSS 색상 키워드는 받는다.
    assert!(ColorValue::parse("transparent").is_some());
    assert!(ColorValue::parse("seagreen").is_some());
}

struct SubDocumentContext;

impl WikiContext for SubDocumentContext {
    fn document_exists(&self, _title: &str) -> bool {
        true
    }

    fn current_title(&self) -> Option<String> {
        Some("알파위키:문법 도움말/심화".to_string())
    }
}

fn link_of(source: &str, context: &dyn WikiContext) -> RenderInline {
    let tree = build_render_tree(&namumark_parser::parse(source), context);
    let RenderBlock::Paragraph { content: inlines } = tree.blocks.into_iter().next().unwrap()
    else {
        panic!("문단이어야 한다");
    };
    inlines.into_iter().next().unwrap()
}

// 렌더확정: `알파위키:문법 도움말/심화`에서 the seed는 `[[../#문법 무효화|…]]`를
// `/w/알파위키:문법 도움말#문법 무효화`로, `[[/TeX|수식]]`을 `…/심화/TeX`로 보낸다.
#[test]
fn relative_links_resolve_against_current_document() {
    assert!(matches!(
        link_of("[[../#문법 무효화|기본편]]", &SubDocumentContext),
        RenderInline::DocumentLink { title, anchor, .. }
            if title == "알파위키:문법 도움말" && anchor.as_deref() == Some("문법 무효화")
    ));
    assert!(matches!(
        link_of("[[/TeX|수식]]", &SubDocumentContext),
        RenderInline::DocumentLink { title, .. } if title == "알파위키:문법 도움말/심화/TeX"
    ));
}

// 렌더확정: the seed는 `[[/심화]]`를 `/심화`로 적힌 대로 보여 준다.
#[test]
fn relative_link_shows_what_was_written() {
    assert!(matches!(
        link_of("[[/TeX]]", &SubDocumentContext),
        RenderInline::DocumentLink { display, .. }
            if display == vec![RenderInline::Text { text: "/TeX".to_string() }]
    ));
}

// 렌더확정: 상위가 없는 문서의 `[[../]]`는 자기 자신으로 가고 `wiki-self-link`가 된다.
// 앵커만 있는 `[[#앵커]]`는 자기 문서라도 자기 링크가 아니다.
#[test]
fn link_to_current_document_is_a_self_link() {
    struct RootContext;
    impl WikiContext for RootContext {
        fn current_title(&self) -> Option<String> {
            Some("알파위키:문법 도움말".to_string())
        }
    }
    assert!(matches!(
        link_of("[[../]]", &RootContext),
        RenderInline::DocumentLink {
            kind: DocumentLinkKind::Current,
            title,
            ..
        } if title == "알파위키:문법 도움말"
    ));
    assert!(matches!(
        link_of("[[#리스트|본 문서 리스트 문단]]", &RootContext),
        RenderInline::DocumentLink {
            kind: DocumentLinkKind::Existing,
            ..
        }
    ));
}

/// layout이 끝난 트리에는 resolve 중간 상태인 [`RenderInline::Footnote`]가 남지 않는다.
///
/// 이 변형은 IR 계약에서 빠져 있어(직렬화하면 오류) 하나라도 새면 프론트엔드로 가는
/// 응답이 통째로 실패한다. 각주 문법 전반을 한 문서에 몰아 넣고 확인한다.
#[test]
fn layout_leaves_no_pending_footnotes() {
    let source = "\
본문[* 무명 각주]과 이름 각주[*A 이름 붙은 것]와 재참조[*A].
[각주]
== 문단 ==
문단 뒤 각주[* 뒤쪽]와 중첩[* 안에 [[링크]]와 [* 더 깊은 것]].
";
    let tree = build_render_tree(&namumark_parser::parse(source), &EmptyContext);

    let mut pending = 0;
    walk_tree(&tree, &mut |inline| {
        if matches!(inline, RenderInline::Footnote { .. }) {
            pending += 1;
        }
    });
    assert_eq!(pending, 0, "layout이 각주 {pending}개를 치환하지 못했다");

    // 각주가 실제로 있었는지도 본다 — 없으면 위 단언이 공허하다.
    let mut references = 0;
    walk_tree(&tree, &mut |inline| {
        if matches!(inline, RenderInline::FootnoteReference { .. }) {
            references += 1;
        }
    });
    assert!(references > 0, "각주 참조가 하나도 만들어지지 않았다");
}

fn walk_inlines(blocks: &[RenderBlock], visit: &mut impl FnMut(&RenderInline)) {
    for block in blocks {
        match block {
            RenderBlock::Heading { content, .. } | RenderBlock::Paragraph { content } => {
                walk_inline_slice(content, visit);
            }
            RenderBlock::Quote { blocks } | RenderBlock::Indent { blocks } => {
                walk_inlines(blocks, visit);
            }
            RenderBlock::List { items, .. } => {
                for item in items {
                    walk_inlines(&item.blocks, visit);
                }
            }
            RenderBlock::Table { table } => {
                if let Some(caption) = &table.caption {
                    walk_inline_slice(caption, visit);
                }
                for row in &table.rows {
                    for cell in &row.cells {
                        walk_inlines(&cell.blocks, visit);
                    }
                }
            }
            RenderBlock::HorizontalRule => {}
        }
    }
}

fn walk_inline_slice(inlines: &[RenderInline], visit: &mut impl FnMut(&RenderInline)) {
    for inline in inlines {
        visit(inline);
        match inline {
            RenderInline::Styled { content, .. }
            | RenderInline::Colored { content, .. }
            | RenderInline::Sized { content, .. }
            | RenderInline::DocumentLink {
                display: content, ..
            }
            | RenderInline::Footnote { content, .. } => walk_inline_slice(content, visit),
            RenderInline::ExternalLink {
                display: Some(display),
                ..
            } => walk_inline_slice(display, visit),
            RenderInline::Blocks { blocks }
            | RenderInline::WikiStyle { blocks, .. }
            | RenderInline::Folding { blocks, .. } => walk_inlines(blocks, visit),
            _ => {}
        }
    }
}

/// 트리 전체 — 블록뿐 아니라 트리 밖에 있는 목차 제목과 각주 내용까지 훑는다.
fn walk_tree(tree: &RenderTree, visit: &mut impl FnMut(&RenderInline)) {
    walk_inlines(&tree.blocks, visit);
    for entry in &tree.table_of_contents {
        walk_inline_slice(&entry.title, visit);
    }
    for footnote in &tree.footnotes {
        walk_inline_slice(&footnote.content, visit);
    }
}
