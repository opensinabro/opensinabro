use namumark_ir::{RenderBlock, RenderInline, RenderTree, TextStyle};
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
    // 문서 끝에 잔여 각주 섹션이 자동 방출된다
    let Some(RenderBlock::FootnoteSection { notes }) = tree.blocks.last() else {
        panic!("문서 끝 각주 섹션이 있어야 한다");
    };
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].label, "1");
    assert_eq!(notes[1].label, "A");
    assert_eq!(notes[1].reference_count, 2);
}

#[test]
fn footnote_macro_flushes_pending_notes() {
    let tree = tree("본문[* 하나]\n[각주]\n다음[* 둘]");
    let section_labels: Vec<Vec<String>> = tree
        .blocks
        .iter()
        .filter_map(|block| match block {
            RenderBlock::FootnoteSection { notes } => {
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

#[test]
fn include_cycle_is_guarded() {
    let mut context = TestContext::new();
    context
        .documents
        .insert("틀:루프".to_string(), "[include(틀:루프)]".to_string());
    let document = namumark_parser::parse("[include(틀:루프)]");
    let tree = build_render_tree(&document, &context);
    // 순환은 Unresolved로 보존되고 무한 확장하지 않는다
    assert!(format!("{:?}", tree.blocks).contains("Unresolved"));
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
        RenderInline::DocumentLink { exists: true, .. }
    ));
    assert!(matches!(
        &inlines[2],
        RenderInline::DocumentLink { exists: false, .. }
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
        ColorValue::Rgb {
            red: 255,
            green: 255,
            blue: 255,
        }
    );
    assert_eq!(
        ColorValue::parse("red"),
        ColorValue::Named("red".to_string())
    );
    assert_eq!(ColorValue::parse("#ff8000").to_string(), "#ff8000");
    assert_eq!(Dimension::Percentage(80).to_string(), "80%");
}
