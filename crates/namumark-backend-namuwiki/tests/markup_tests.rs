//! 나무위키 동등 마크업 방출 검증.

use namumark_backend_namuwiki::{NamuwikiMarkup, namuwiki_markup, stylesheet};
use namumark_ir::RenderBackend;
use namumark_render::EmptyContext;

fn render(source: &str) -> String {
    let document = namumark_parser::parse(source);
    let tree = namumark_render::build_render_tree(&document, &EmptyContext);
    NamuwikiMarkup.render(&tree)
}

#[test]
fn table_of_contents_macro_renders_all_headings() {
    let markup = render("[목차]\n== 제목 ==\n내용");
    assert!(markup.contains("wiki-macro-toc"));
    assert!(markup.contains("href=\"#s-1\""));
    assert!(markup.contains("id=\"s-1\""));
}

#[test]
fn text_is_escaped() {
    let markup = render("<script>alert(1)</script>");
    assert!(!markup.contains("<script>"));
    assert!(markup.contains("&lt;script&gt;"));
}

// 라이트 색은 style로, 다크 색은 data-dark-style로 — 나무위키 표기다.
#[test]
fn dual_color_splits_into_style_and_dark_style() {
    let markup = render("{{{#ff0000,#00ff00 듀얼}}}");
    assert!(markup.contains(r#"style="color:#ff0000""#), "{markup}");
    assert!(
        markup.contains(r#"data-dark-style="color:#00ff00;""#),
        "{markup}"
    );
}

// 다크 색을 따로 주지 않아도 나무위키는 같은 값으로 채운다.
#[test]
fn single_color_fills_dark_style_with_same_value() {
    let markup = render("{{{#ff0000 하나}}}");
    assert!(markup.contains(r#"style="color:#ff0000""#), "{markup}");
    assert!(
        markup.contains(r#"data-dark-style="color:#ff0000;""#),
        "{markup}"
    );
}

#[test]
fn markup_streams_as_display() {
    let document = namumark_parser::parse("'''굵게'''");
    let tree = namumark_render::build_render_tree(&document, &EmptyContext);
    let markup = format!("{}", namuwiki_markup(&tree));
    assert!(markup.contains("<strong>굵게</strong>"));
}

#[test]
fn stylesheet_is_bundled() {
    assert!(stylesheet().contains(".wiki-paragraph"));
}
