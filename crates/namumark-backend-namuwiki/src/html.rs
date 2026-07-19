//! 걸러 낸 `#!html` 트리를 나무위키 표기로 방출한다.
//!
//! 무엇을 통과시킬지는 IR([`namumark_ir::HtmlNode`])이 이미 정했다. 여기서는 표현
//! 정책만 얹는다 — 위키 입력이 만든 링크를 나무마크 외부 링크와 똑같이 꾸며서,
//! 새 창으로 열고 검색 엔진이 따라가지 않으며 연 문서의 창 객체를 넘기지 않게 한다.

use crate::StyleDeclarations;
use crate::tag::{escape_text, tag};
use namumark_ir::{HtmlNode, HtmlTag};
use std::fmt::{self, Display, Formatter};

pub(crate) struct HtmlMarkup<'nodes>(pub(crate) &'nodes [HtmlNode]);

impl Display for HtmlMarkup<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for node in self.0 {
            write_node(formatter, node)?;
        }
        Ok(())
    }
}

fn write_node(formatter: &mut Formatter<'_>, node: &HtmlNode) -> fmt::Result {
    match node {
        HtmlNode::Text { text } => write!(formatter, "{}", escape_text(text)),
        HtmlNode::Element {
            tag: html_tag,
            attributes,
            children,
        } => {
            let is_link = *html_tag == HtmlTag::Anchor;
            let mut element = tag(formatter, html_tag.name())?;
            // 링크 클래스는 우리가 정한다 — 위키 입력이 UI 클래스를 사칭하지 못한다.
            element = element
                .attribute_if_some(
                    "class",
                    attributes
                        .class
                        .as_ref()
                        .filter(|_| !is_link)
                        .map(|value| value as &dyn Display),
                )?
                .attribute_if_some(
                    "href",
                    attributes.href.as_ref().map(|value| value as &dyn Display),
                )?
                .attribute_when(
                    !attributes.style.is_empty(),
                    "style",
                    &StyleDeclarations(&attributes.style),
                )?
                .attribute_if_some(
                    "src",
                    attributes
                        .source
                        .as_ref()
                        .map(|value| value as &dyn Display),
                )?
                .attribute_if_some(
                    "width",
                    attributes.width.as_ref().map(|value| value as &dyn Display),
                )?
                .attribute_if_some(
                    "height",
                    attributes
                        .height
                        .as_ref()
                        .map(|value| value as &dyn Display),
                )?;
            // the seed도 `<video controls>`를 `controls=""`로 낸다.
            if attributes.controls {
                element = element.attribute("controls", &"")?;
            }
            if is_link {
                element = element
                    .attribute("target", &"_blank")?
                    .attribute("rel", &"nofollow noopener ugc")?
                    .attribute("class", &"wiki-link-external")?;
            }
            if html_tag.is_void() {
                return element.void();
            }
            element.content(|formatter| {
                for child in children {
                    write_node(formatter, child)?;
                }
                Ok(())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(source: &str) -> String {
        HtmlMarkup(&HtmlNode::parse(source)).to_string()
    }

    #[test]
    fn keeps_allowed_markup() {
        assert_eq!(
            render(r#"<span style="background-color: #999">배경색 적용</span>"#),
            r#"<span style="background-color: #999">배경색 적용</span>"#
        );
        assert_eq!(render("<b>굵게</b>"), "<b>굵게</b>");
        assert_eq!(render("줄<br>바꿈"), "줄<br>바꿈");
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(render("&nbsp;"), "\u{a0}");
        assert_eq!(render("&#8203"), "\u{200b}");
    }

    // 디코딩한 글자도 태그가 되어선 안 된다.
    #[test]
    fn decoded_entities_cannot_become_tags() {
        assert_eq!(
            render("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(render("&#60;script&#62;"), "&lt;script&gt;");
        // 되살아난 `&`는 다시 엔티티를 이루지 못한다 (렌더확정: `&amp;nbsp;`는 그대로다).
        assert_eq!(render("&amp;nbsp;"), "&amp;nbsp;");
    }

    #[test]
    fn drops_script_with_its_content() {
        assert_eq!(render("앞<script>alert(1)</script>뒤"), "앞뒤");
        assert_eq!(render("<style>body{display:none}</style>"), "");
    }

    #[test]
    fn unwraps_unknown_tags() {
        assert_eq!(render("<marquee>글</marquee>"), "글");
        assert_eq!(render("<img src='//evil'>"), "");
    }

    // 위키 입력이 만든 링크도 나무마크 외부 링크와 같은 수준으로 나간다.
    #[test]
    fn keeps_web_links_and_hardens_them() {
        assert_eq!(
            render("<a href='https://namu.wiki/w/X'>글</a>"),
            "<a href=\"https://namu.wiki/w/X\" target=\"_blank\" \
             rel=\"nofollow noopener ugc\" class=\"wiki-link-external\">글</a>"
        );
    }

    // 위키 입력이 링크 클래스를 제 마음대로 정하지 못한다.
    #[test]
    fn link_class_is_ours() {
        assert_eq!(
            render("<a class='evil' href='https://x.test/'>글</a>"),
            "<a href=\"https://x.test/\" target=\"_blank\" \
             rel=\"nofollow noopener ugc\" class=\"wiki-link-external\">글</a>"
        );
    }

    #[test]
    fn drops_event_handlers_and_unknown_attributes() {
        assert_eq!(
            render(r#"<span onerror="alert(1)" onclick="x()">글</span>"#),
            "<span>글</span>"
        );
        assert_eq!(render(r#"<div id="s-1">글</div>"#), "<div>글</div>");
    }

    #[test]
    fn drops_style_that_can_call_code() {
        assert_eq!(
            render(r#"<div style="background: url(javascript:alert(1))">글</div>"#),
            "<div>글</div>"
        );
        assert_eq!(
            render(r#"<div style="@import 'evil.css'">글</div>"#),
            "<div>글</div>"
        );
    }

    #[test]
    fn closes_dangling_tags() {
        assert_eq!(render("<b>안 닫음"), "<b>안 닫음</b>");
        assert_eq!(render("</b>짝 없는 닫기"), "짝 없는 닫기");
    }

    #[test]
    fn escapes_stray_angle_brackets() {
        assert_eq!(render("3 < 5"), "3 &lt; 5");
        assert_eq!(render("<span>3 > 2</span>"), "<span>3 &gt; 2</span>");
    }

    #[test]
    fn escapes_quotes_in_attribute_values() {
        assert_eq!(
            render(r#"<span class='a"><script>alert(1)</script>'>글</span>"#),
            r#"<span class="a&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;">글</span>"#
        );
    }
}
