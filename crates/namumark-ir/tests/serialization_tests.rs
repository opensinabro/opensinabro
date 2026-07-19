//! IR의 와이어 표현을 고정한다.
//!
//! IR은 프론트엔드와의 계약이다. 생성된 TypeScript 타입(`frontend/lib/namumark`)이
//! 모양을 말해 주지만, 그 타입이 실제 JSON과 같다는 보장은 여기서만 나온다 —
//! 판별자 이름, 필드 표기, 값 타입이 문자열로 나가는 것까지.

use namumark_ir::{
    Color, ColorValue, Dimension, HtmlAttributes, HtmlNode, HtmlTag, ImageLayout, RenderBlock,
    RenderInline, RenderTree, StyleDeclaration, TableStyleProperty, TextStyle,
};

fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("IR은 항상 직렬화된다")
}

/// 변형은 `type` 판별자로 갈린다. `kind`가 아닌 이유는 `List`·`DocumentLink`가
/// 이미 `kind` 필드를 쓰기 때문이다.
#[test]
fn variants_are_tagged_with_type() {
    assert_eq!(
        json(&RenderBlock::Paragraph {
            content: vec![RenderInline::Text {
                text: "글".to_string()
            }],
        }),
        serde_json::json!({
            "type": "paragraph",
            "content": [{ "type": "text", "text": "글" }],
        })
    );
    assert_eq!(
        json(&RenderBlock::HorizontalRule),
        serde_json::json!({ "type": "horizontalRule" })
    );
}

#[test]
fn fields_go_out_in_camel_case() {
    assert_eq!(
        json(&RenderInline::Image {
            file_name: "표범.png".to_string(),
            url: None,
            layout: ImageLayout::default(),
        })["fileName"],
        serde_json::json!("표범.png")
    );
}

/// 값 타입은 성분이 아니라 CSS 표기 그대로 나간다 — 받는 쪽이 다시 조립할 일이 없다.
#[test]
fn value_types_go_out_as_css_text() {
    assert_eq!(
        json(&Color {
            light: ColorValue::Rgb {
                red: 255,
                green: 0,
                blue: 0
            },
            dark: None,
        }),
        serde_json::json!({ "light": "#ff0000", "dark": null })
    );
    assert_eq!(
        json(&TableStyleProperty::Width {
            width: Dimension::Percentage(50)
        }),
        serde_json::json!({ "type": "width", "width": "50%" })
    );
}

/// 이름만 있는 열거형은 문자열 하나로 나간다.
#[test]
fn unit_enums_go_out_as_strings() {
    assert_eq!(
        json(&TextStyle::Strikethrough),
        serde_json::json!("strikethrough")
    );
    assert_eq!(
        json(&HtmlTag::WordBreakOpportunity),
        serde_json::json!("wordBreakOpportunity")
    );
}

/// `#!html`은 문자열이 아니라 트리로 나간다 — 받는 쪽이 HTML을 다시 파싱하거나
/// 통째로 주입할 필요가 없다.
#[test]
fn filtered_html_goes_out_as_a_tree() {
    let nodes = HtmlNode::parse(r#"<span style="color: red">글</span>"#);
    assert_eq!(
        json(&RenderInline::Html { nodes }),
        serde_json::json!({
            "type": "html",
            "nodes": [{
                "type": "element",
                "tag": "span",
                "attributes": {
                    "class": null,
                    "href": null,
                    "style": [{ "property": "color", "value": "red" }],
                    "source": null,
                    "width": null,
                    "height": null,
                    "controls": false,
                },
                "children": [{ "type": "text", "text": "글" }],
            }],
        })
    );
}

#[test]
fn tree_carries_redirect_and_categories() {
    assert_eq!(
        json(&RenderTree {
            redirect: Some("다른 문서".to_string()),
            blocks: Vec::new(),
            categories: vec!["분류".to_string()],
            table_of_contents: Vec::new(),
            footnotes: Vec::new(),
        }),
        serde_json::json!({
            "redirect": "다른 문서",
            "blocks": [],
            "categories": ["분류"],
            "tableOfContents": [],
            "footnotes": [],
        })
    );
}

#[test]
fn style_declarations_keep_their_written_form() {
    assert_eq!(
        json(&StyleDeclaration {
            property: "background-color".to_string(),
            value: "#999".to_string(),
        }),
        serde_json::json!({ "property": "background-color", "value": "#999" })
    );
}

#[test]
fn html_attributes_are_one_slot_each() {
    assert_eq!(
        json(&HtmlAttributes::default())["controls"],
        serde_json::json!(false)
    );
}
