use namumark_ir::{DocumentLinkKind, RenderBlock, RenderInline, RenderTree, TextStyle};
use namumark_render::{Date, DateTime, EmptyContext, Time, WikiContext, build_render_tree};
use std::collections::HashMap;

struct TestContext {
    documents: HashMap<String, String>,
    now: Option<DateTime>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
            now: None,
        }
    }
}

impl WikiContext for TestContext {
    fn document_exists(&self, title: &str) -> bool {
        self.documents.contains_key(title)
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

#[test]
fn footnotes_are_numbered_and_merged() {
    let tree = tree("본문[* 첫째][*A 이름 각주][*A] 끝");
    // 문서 끝에 잔여 각주 섹션이 자동 방출된다. `[각주]`는 매크로라 문단 안에 놓인다.
    let Some(RenderBlock::Paragraph(inlines)) = tree.blocks.last() else {
        panic!("문서 끝 각주 문단이 있어야 한다");
    };
    let Some(RenderInline::FootnoteSection { notes }) = inlines.first() else {
        panic!("각주 섹션이 있어야 한다: {inlines:?}");
    };
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].label, "1");
    assert_eq!(notes[1].label, "A");
    assert_eq!(notes[1].reference_numbers.len(), 2);
}

#[test]
fn footnote_macro_flushes_pending_notes() {
    let tree = tree("본문[* 하나]\n[각주]\n다음[* 둘]");
    let section_labels: Vec<Vec<String>> = tree
        .blocks
        .iter()
        .flat_map(|block| match block {
            RenderBlock::Paragraph(inlines) => inlines.as_slice(),
            _ => &[],
        })
        .filter_map(|inline| match inline {
            RenderInline::FootnoteSection { notes } => {
                Some(notes.iter().map(|note| note.label.clone()).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        section_labels,
        vec![vec!["1".to_string()], vec!["2".to_string()]]
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
    let RenderBlock::Paragraph(inlines) = &tree.blocks[0] else {
        panic!("문단이어야 한다");
    };
    assert_eq!(
        inlines[0],
        RenderInline::Styled {
            style: TextStyle::Bold,
            content: vec![RenderInline::Text("단풍".to_string())],
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
    let RenderBlock::Paragraph(inlines) = tree.blocks.last().unwrap() else {
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
    let RenderBlock::Paragraph(inlines) = &tree.blocks[0] else {
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
    let RenderBlock::Paragraph(inlines) = &tree.blocks[0] else {
        panic!("문단이어야 한다");
    };
    assert_eq!(inlines[0], RenderInline::Text("26".to_string()));
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
    let RenderBlock::Paragraph(inlines) = tree.blocks.into_iter().next().unwrap() else {
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
            if display == vec![RenderInline::Text("/TeX".to_string())]
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
