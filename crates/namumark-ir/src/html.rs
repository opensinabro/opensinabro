//! `{{{#!html}}}` 원문을 걸러 낸 결과.
//!
//! 나무마크는 위키 사용자가 쓰는 입력이고 `#!html`은 그 입력이 원시 HTML로 나가는
//! 유일한 통로다. 그래서 **화이트리스트**로 간다 — 아는 것만 통과시키고 나머지는
//! 버린다. 모르는 태그·속성이 생겼을 때 통과되는 쪽이 아니라 막히는 쪽이어야 한다.
//!
//! 걸러 낸 결과를 문자열이 아니라 [`HtmlNode`] 트리로 담는 이유는 화이트리스트를
//! 타입에 가두기 위해서다. 허용하지 않는 태그·속성은 표현할 방법이 없으므로,
//! 백엔드는 자기가 받은 트리가 이미 걸러졌다는 것을 타입으로 안다 — 백엔드마다
//! 정제를 되풀이하거나 빠뜨릴 여지가 없다.
//!
//! 나무위키는 `#!html`을 원시 HTML로 렌더한다(`{{{#!html <span style="…">글</span>}}}` →
//! `<span style="background-color:#999">글</span>`, `&nbsp;` → U+00A0). 다만 나무위키가
//! 정확히 무엇까지 허용하는지는 문서에 위험한 예제가 없어 렌더 대조로 확정할 수 없다.
//! 문법 도움말도 `#!html`을 "비권장, 지원 종료 가능"이라 서술한다. 그래서 허용 범위는
//! 도움말이 실제로 보여주는 표현(텍스트 서식, `<span class>`, `<div style>`)으로 좁힌다.

use crate::StyleDeclaration;
use serde::Serialize;
use ts_rs::TS;

/// 걸러 낸 `#!html`의 노드 하나.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum HtmlNode {
    /// 문자 참조가 이미 글자로 풀린 본문. 이스케이프는 방출부가 맡는다.
    Text { text: String },
    Element {
        tag: HtmlTag,
        attributes: HtmlAttributes,
        children: Vec<HtmlNode>,
    },
}

/// 통과시키는 태그. 텍스트 서식과 문단 구조에 더해 `<video>`까지다(렌더확정: `틀:video`의
/// `{{{#!html <video src="…" …></video>}}}`가 the seed에서 실제 `<video>`로 나온다).
/// 스크립트·삽입·폼은 없다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HtmlTag {
    Anchor,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Strong,
    Emphasis,
    Subscript,
    Superscript,
    LineBreak,
    Span,
    Division,
    Code,
    Small,
    Big,
    WordBreakOpportunity,
    Video,
}

impl HtmlTag {
    pub fn name(self) -> &'static str {
        match self {
            HtmlTag::Anchor => "a",
            HtmlTag::Bold => "b",
            HtmlTag::Italic => "i",
            HtmlTag::Underline => "u",
            HtmlTag::Strikethrough => "s",
            HtmlTag::Strong => "strong",
            HtmlTag::Emphasis => "em",
            HtmlTag::Subscript => "sub",
            HtmlTag::Superscript => "sup",
            HtmlTag::LineBreak => "br",
            HtmlTag::Span => "span",
            HtmlTag::Division => "div",
            HtmlTag::Code => "code",
            HtmlTag::Small => "small",
            HtmlTag::Big => "big",
            HtmlTag::WordBreakOpportunity => "wbr",
            HtmlTag::Video => "video",
        }
    }

    fn parse(name: &str) -> Option<HtmlTag> {
        Some(match name {
            "a" => HtmlTag::Anchor,
            "b" => HtmlTag::Bold,
            "i" => HtmlTag::Italic,
            "u" => HtmlTag::Underline,
            "s" => HtmlTag::Strikethrough,
            "strong" => HtmlTag::Strong,
            "em" => HtmlTag::Emphasis,
            "sub" => HtmlTag::Subscript,
            "sup" => HtmlTag::Superscript,
            "br" => HtmlTag::LineBreak,
            "span" => HtmlTag::Span,
            "div" => HtmlTag::Division,
            "code" => HtmlTag::Code,
            "small" => HtmlTag::Small,
            "big" => HtmlTag::Big,
            "wbr" => HtmlTag::WordBreakOpportunity,
            "video" => HtmlTag::Video,
            _ => return None,
        })
    }

    /// 내용이 없는 태그.
    pub fn is_void(self) -> bool {
        matches!(self, HtmlTag::LineBreak | HtmlTag::WordBreakOpportunity)
    }
}

/// 통과시키는 속성. 자리마다 값이 하나씩이므로 같은 속성을 두 번 쓴 입력은
/// 먼저 쓴 값만 남는다 — 브라우저가 하는 것과 같다.
///
/// `id`는 문서 앵커(`s-1`, `fn-1`)와 충돌할 수 있어 받지 않는다 — 이 구조체에 자리가
/// 없다는 것이 곧 그 보장이다. 이미지의 `src`도 나무마크 이미지 문법이 대신하므로
/// [`HtmlAttributes::source`]는 `<video>`에만 붙는다.
///
/// `href`는 연다 — 나무위키가 그렇게 하고(렌더확정: `틀:문서 가져옴/나무위키`의
/// `{{{#!html <a href="https://namu.wiki/w/…">…</a>}}}`가 실제로 링크로 나온다),
/// 웹 주소만 통과하므로 여기 담긴 값은 이미 `http`·`https`·`ftp`다.
///
/// `target`·`rel`은 없다. 위키 입력이 정할 것이 아니라 렌더러의 표현 정책이다.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HtmlAttributes {
    pub class: Option<String>,
    pub href: Option<String>,
    pub style: Vec<StyleDeclaration>,
    pub source: Option<String>,
    /// `<video>`의 표시 속성. CSS 길이가 아니라 HTML 속성이라 표기를 그대로 둔다.
    pub width: Option<String>,
    pub height: Option<String>,
    pub controls: bool,
}

/// 열어 둔 요소 하나. 닫히면 자식으로 접힌다.
struct OpenElement {
    tag: HtmlTag,
    attributes: HtmlAttributes,
    children: Vec<HtmlNode>,
}

impl HtmlNode {
    /// 허용 목록에 없는 태그·속성은 버리고, 텍스트는 살린다.
    ///
    /// 태그를 버릴 때 그 안의 텍스트는 남긴다(껍데기만 벗긴다) — 사용자가 쓴 글이
    /// 통째로 사라지는 편보다 낫다. 다만 `<script>`·`<style>`처럼 **내용 자체가 코드**인
    /// 태그는 내용까지 버린다.
    pub fn parse(source: &str) -> Vec<HtmlNode> {
        let mut roots = Vec::new();
        let mut open: Vec<OpenElement> = Vec::new();
        let mut rest = source;

        while let Some(index) = rest.find('<') {
            push_text(&rest[..index], &mut open, &mut roots);
            rest = &rest[index..];

            let Some(end) = find_tag_end(rest) else {
                // 닫히지 않은 `<`는 글자일 뿐이다.
                push_text(rest, &mut open, &mut roots);
                return close_all(open, roots);
            };
            let inner = &rest[1..end];
            rest = &rest[end + 1..];

            if let Some(name) = inner.strip_prefix('/') {
                let name = name.trim().to_ascii_lowercase();
                // 우리가 연 적이 있는 태그만 닫는다. 짝이 안 맞는 닫기는 버린다.
                if open.last().map(|element| element.tag.name()) == Some(name.as_str()) {
                    let element = open.pop().expect("바로 위에서 확인했다");
                    let node = HtmlNode::Element {
                        tag: element.tag,
                        attributes: element.attributes,
                        children: element.children,
                    };
                    push_node(node, &mut open, &mut roots);
                }
                continue;
            }
            if inner.starts_with('!') {
                continue; // 주석·doctype
            }

            let inner = inner.strip_suffix('/').unwrap_or(inner);
            let (name, attribute_source) = split_tag_name(inner);
            let name = name.to_ascii_lowercase();

            if is_code_bearing(&name) {
                rest = skip_until_close(rest, &name);
                continue;
            }
            let Some(tag) = HtmlTag::parse(&name) else {
                continue; // 껍데기만 버리고 내용은 계속 읽는다
            };
            let attributes = safe_attributes(tag, attribute_source);
            // 갈 곳이 없는 링크는 링크가 아니다 — 껍데기를 벗기고 글만 남긴다.
            if tag == HtmlTag::Anchor && attributes.href.is_none() {
                continue;
            }

            if tag.is_void() {
                push_node(
                    HtmlNode::Element {
                        tag,
                        attributes,
                        children: Vec::new(),
                    },
                    &mut open,
                    &mut roots,
                );
            } else {
                open.push(OpenElement {
                    tag,
                    attributes,
                    children: Vec::new(),
                });
            }
        }
        push_text(rest, &mut open, &mut roots);
        close_all(open, roots)
    }
}

/// 열어 둔 태그는 우리가 닫는다 — 새어 나가면 바깥 문서 구조가 무너진다.
fn close_all(mut open: Vec<OpenElement>, mut roots: Vec<HtmlNode>) -> Vec<HtmlNode> {
    while let Some(element) = open.pop() {
        let node = HtmlNode::Element {
            tag: element.tag,
            attributes: element.attributes,
            children: element.children,
        };
        push_node(node, &mut open, &mut roots);
    }
    roots
}

fn push_node(node: HtmlNode, open: &mut [OpenElement], roots: &mut Vec<HtmlNode>) {
    match open.last_mut() {
        Some(element) => element.children.push(node),
        None => roots.push(node),
    }
}

/// 문자 참조를 글자로 풀어 본문으로 넣는다. 껍데기를 벗긴 태그가 본문을 조각내므로
/// 바로 앞이 본문이면 이어 붙인다 — 트리 모양이 태그 유무로 흔들리지 않게 한다.
fn push_text(text: &str, open: &mut [OpenElement], roots: &mut Vec<HtmlNode>) {
    if text.is_empty() {
        return;
    }
    let decoded = decode_entities(text);
    let siblings = match open.last_mut() {
        Some(element) => &mut element.children,
        None => roots,
    };
    match siblings.last_mut() {
        Some(HtmlNode::Text { text }) => text.push_str(&decoded),
        _ => siblings.push(HtmlNode::Text { text: decoded }),
    }
}

/// 내용 자체가 코드라 껍데기만 벗기면 위험한 태그.
fn is_code_bearing(name: &str) -> bool {
    matches!(name, "script" | "style" | "iframe" | "object" | "embed")
}

fn skip_until_close<'source>(rest: &'source str, name: &str) -> &'source str {
    let closing = format!("</{name}");
    match rest.to_ascii_lowercase().find(&closing) {
        Some(index) => match rest[index..].find('>') {
            Some(end) => &rest[index + end + 1..],
            None => "",
        },
        None => "",
    }
}

fn find_tag_end(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut position = 1;
    let mut quote = None;
    while position < bytes.len() {
        let byte = bytes[position];
        match (quote, byte) {
            (None, b'"') | (None, b'\'') => quote = Some(byte),
            (Some(open), _) if byte == open => quote = None,
            (None, b'>') => return Some(position),
            (None, b'<') => return None, // 닫히지 않은 `<`
            _ => {}
        }
        position += 1;
    }
    None
}

fn split_tag_name(inner: &str) -> (&str, &str) {
    match inner.find(|character: char| character.is_whitespace()) {
        Some(index) => (&inner[..index], &inner[index..]),
        None => (inner, ""),
    }
}

fn safe_attributes(tag: HtmlTag, source: &str) -> HtmlAttributes {
    let mut attributes = HtmlAttributes::default();
    for (name, value) in parse_attributes(source) {
        // 자리마다 값은 하나뿐이다 — 먼저 쓴 값만 남긴다.
        let slot = match (tag, name.as_str()) {
            (_, "class") => &mut attributes.class,
            (_, "style") => {
                if attributes.style.is_empty()
                    && let Some(declarations) = StyleDeclaration::parse_html_attribute(&value)
                {
                    attributes.style = declarations;
                }
                continue;
            }
            // `href`는 링크에만 뜻이 있다.
            (HtmlTag::Anchor, "href") => &mut attributes.href,
            // 미디어 속성은 `<video>` 전용이다.
            (HtmlTag::Video, "src") => &mut attributes.source,
            (HtmlTag::Video, "width") => &mut attributes.width,
            (HtmlTag::Video, "height") => &mut attributes.height,
            (HtmlTag::Video, "controls") => {
                attributes.controls = true;
                continue;
            }
            _ => continue,
        };
        if slot.is_none() {
            // 주소 자리는 링크와 같은 수준으로 깎는다.
            *slot = match name.as_str() {
                "href" | "src" => safe_link(&value),
                _ => Some(value),
            };
        }
    }
    attributes
}

/// 링크 주소는 웹 주소만 받는다. `javascript:`·`data:`처럼 코드를 부르는 스킴은 물론,
/// 스킴이 아예 없는 상대 주소도 받지 않는다 — 위키 안 링크는 나무마크 문법이 대신한다.
fn safe_link(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let lowered = trimmed.to_ascii_lowercase();
    ["http://", "https://", "ftp://"]
        .iter()
        .any(|scheme| lowered.starts_with(scheme))
        .then(|| trimmed.to_string())
}

fn parse_attributes(source: &str) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    let bytes = source.as_bytes();
    let mut position = 0;
    while position < bytes.len() {
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        let start = position;
        while position < bytes.len()
            && !bytes[position].is_ascii_whitespace()
            && bytes[position] != b'='
        {
            position += 1;
        }
        if start == position {
            break;
        }
        let name = source[start..position].to_ascii_lowercase();
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        if position >= bytes.len() || bytes[position] != b'=' {
            // 값 없는 속성은 빈 값으로 받는다(the seed도 `<video controls>`를 `controls=""`로
            // 낸다). 화이트리스트에 없으면 뒤의 필터가 어차피 걸러 낸다.
            attributes.push((name, String::new()));
            continue;
        }
        position += 1;
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        if position >= bytes.len() {
            break;
        }
        let value = if bytes[position] == b'"' || bytes[position] == b'\'' {
            let quote = bytes[position];
            position += 1;
            let start = position;
            while position < bytes.len() && bytes[position] != quote {
                position += 1;
            }
            let value = source[start..position].to_string();
            position += 1;
            value
        } else {
            let start = position;
            while position < bytes.len() && !bytes[position].is_ascii_whitespace() {
                position += 1;
            }
            source[start..position].to_string()
        };
        attributes.push((name, value));
    }
    attributes
}

/// 본문의 문자 참조를 실제 글자로 바꾼다.
///
/// 나무위키는 `#!html`의 엔티티를 글자로 바꿔 내보낸다(`&nbsp;` → U+00A0,
/// `&#8203` → U+200B). 세미콜론이 빠져도 브라우저처럼 받아 준다 — 실제 문서가
/// `{{{#!html &#8203}}}`처럼 쓴다.
///
/// 풀어 낸 글자가 태그가 되지 못하게 막는 일은 방출부의 이스케이프가 맡는다.
/// 여기서 나온 값은 글자일 뿐 마크업이 아니다.
fn decode_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        decoded.push_str(&rest[..index]);
        rest = &rest[index..];
        match decode_entity(rest) {
            Some((character, length)) => {
                decoded.push(character);
                rest = &rest[length..];
            }
            None => {
                decoded.push('&');
                rest = &rest[1..];
            }
        }
    }
    decoded.push_str(rest);
    decoded
}

/// `&`로 시작하는 문자 참조를 읽는다. 돌려주는 길이는 `&`를 포함한 표기 전체다.
fn decode_entity(source: &str) -> Option<(char, usize)> {
    let body = source.strip_prefix('&')?;
    // 이름/숫자 뒤의 `;`는 있으면 표기에 포함하고, 없어도 받는다.
    let end = body
        .char_indices()
        .find(|(_, character)| !character.is_ascii_alphanumeric() && *character != '#')
        .map(|(index, _)| index)
        .unwrap_or(body.len());
    let name = &body[..end];
    if name.is_empty() {
        return None;
    }
    let length = 1 + end + usize::from(body[end..].starts_with(';'));
    let character = if let Some(digits) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
        char::from_u32(u32::from_str_radix(digits, 16).ok()?)?
    } else if let Some(digits) = name.strip_prefix('#') {
        char::from_u32(digits.parse().ok()?)?
    } else {
        named_entity(name)?
    };
    Some((character, length))
}

/// 실제 문서가 `#!html`에 쓰는 이름 있는 문자 참조.
fn named_entity(name: &str) -> Option<char> {
    Some(match name {
        "nbsp" => '\u{a0}',
        "ZeroWidthSpace" => '\u{200b}',
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "shy" => '\u{ad}',
        "middot" => '\u{b7}',
        "hellip" => '\u{2026}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "commat" => '@',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> HtmlNode {
        HtmlNode::Text {
            text: value.to_string(),
        }
    }

    fn element(tag: HtmlTag, attributes: HtmlAttributes, children: Vec<HtmlNode>) -> HtmlNode {
        HtmlNode::Element {
            tag,
            attributes,
            children,
        }
    }

    #[test]
    fn keeps_allowed_markup() {
        assert_eq!(
            HtmlNode::parse(r#"<span style="background-color: #999">배경색 적용</span>"#),
            vec![element(
                HtmlTag::Span,
                HtmlAttributes {
                    style: vec![StyleDeclaration {
                        property: "background-color".to_string(),
                        value: "#999".to_string(),
                    }],
                    ..HtmlAttributes::default()
                },
                vec![text("배경색 적용")],
            )]
        );
        assert_eq!(
            HtmlNode::parse("<b>굵게</b>"),
            vec![element(
                HtmlTag::Bold,
                HtmlAttributes::default(),
                vec![text("굵게")]
            )]
        );
    }

    #[test]
    fn void_tags_take_no_children() {
        assert_eq!(
            HtmlNode::parse("줄<br>바꿈"),
            vec![
                text("줄"),
                element(HtmlTag::LineBreak, HtmlAttributes::default(), Vec::new()),
                text("바꿈"),
            ]
        );
    }

    // 나무위키는 엔티티를 실제 글자로 바꿔 내보낸다.
    #[test]
    fn decodes_entities() {
        assert_eq!(HtmlNode::parse("&nbsp;"), vec![text("\u{a0}")]);
        assert_eq!(HtmlNode::parse("&ZeroWidthSpace;"), vec![text("\u{200b}")]);
        assert_eq!(HtmlNode::parse("&#8203;"), vec![text("\u{200b}")]);
        // 실제 문서는 세미콜론을 빼먹기도 한다 — 브라우저처럼 받아 준다.
        assert_eq!(HtmlNode::parse("&#8203"), vec![text("\u{200b}")]);
        assert_eq!(HtmlNode::parse("&#x200b;"), vec![text("\u{200b}")]);
    }

    // 디코딩한 글자는 글자일 뿐이다 — 트리에 태그로 들어오지 않는다.
    #[test]
    fn decoded_entities_cannot_become_tags() {
        assert_eq!(
            HtmlNode::parse("&lt;script&gt;alert(1)&lt;/script&gt;"),
            vec![text("<script>alert(1)</script>")]
        );
    }

    #[test]
    fn keeps_unknown_ampersand_as_is() {
        assert_eq!(HtmlNode::parse("A & B"), vec![text("A & B")]);
        assert_eq!(HtmlNode::parse("&unknown;"), vec![text("&unknown;")]);
    }

    #[test]
    fn drops_script_with_its_content() {
        assert_eq!(
            HtmlNode::parse("앞<script>alert(1)</script>뒤"),
            vec![text("앞뒤")]
        );
        assert_eq!(HtmlNode::parse("<style>body{display:none}</style>"), vec![]);
        assert_eq!(HtmlNode::parse("<iframe src='//evil'></iframe>"), vec![]);
    }

    // 허용하지 않는 태그는 껍데기만 벗기고 글은 남긴다.
    #[test]
    fn unwraps_unknown_tags() {
        assert_eq!(HtmlNode::parse("<marquee>글</marquee>"), vec![text("글")]);
        assert_eq!(HtmlNode::parse("<img src='//evil'>"), vec![]);
        // 껍데기를 벗겨도 본문은 한 조각으로 남는다.
        assert_eq!(
            HtmlNode::parse("앞<marquee>가운데</marquee>뒤"),
            vec![text("앞가운데뒤")]
        );
    }

    #[test]
    fn keeps_web_links() {
        assert_eq!(
            HtmlNode::parse("<a href='https://namu.wiki/w/X'>글</a>"),
            vec![element(
                HtmlTag::Anchor,
                HtmlAttributes {
                    href: Some("https://namu.wiki/w/X".to_string()),
                    ..HtmlAttributes::default()
                },
                vec![text("글")],
            )]
        );
    }

    // 갈 곳이 없거나 코드를 부르는 링크는 껍데기를 벗기고 글만 남긴다.
    #[test]
    fn unwraps_links_that_are_not_web_addresses() {
        for source in [
            "<a href='javascript:alert(1)'>글</a>",
            "<a href='data:text/html,<script>'>글</a>",
            "<a href='//evil'>글</a>",
            "<a href='/w/문서'>글</a>",
            "<a>글</a>",
            // 스킴을 대소문자로 감춰도 소용없다.
            "<a href='JaVaScRiPt:alert(1)'>글</a>",
        ] {
            assert_eq!(HtmlNode::parse(source), vec![text("글")], "{source}");
        }
    }

    #[test]
    fn drops_event_handlers_and_unknown_attributes() {
        assert_eq!(
            HtmlNode::parse(r#"<span onerror="alert(1)" onclick="x()">글</span>"#),
            vec![element(
                HtmlTag::Span,
                HtmlAttributes::default(),
                vec![text("글")]
            )]
        );
        // `id`는 문서 앵커와 충돌한다 — 열거형에 자리가 없다.
        assert_eq!(
            HtmlNode::parse(r#"<div id="s-1">글</div>"#),
            vec![element(
                HtmlTag::Division,
                HtmlAttributes::default(),
                vec![text("글")]
            )]
        );
    }

    #[test]
    fn media_attributes_belong_to_video() {
        assert_eq!(
            HtmlNode::parse(r#"<video src="https://x.test/a.mp4" width="640" controls></video>"#),
            vec![element(
                HtmlTag::Video,
                HtmlAttributes {
                    source: Some("https://x.test/a.mp4".to_string()),
                    width: Some("640".to_string()),
                    controls: true,
                    ..HtmlAttributes::default()
                },
                Vec::new(),
            )]
        );
        // `<video>`가 아닌 태그의 미디어 속성은 받지 않는다.
        assert_eq!(
            HtmlNode::parse(r#"<span width="640">글</span>"#),
            vec![element(
                HtmlTag::Span,
                HtmlAttributes::default(),
                vec![text("글")]
            )]
        );
    }

    #[test]
    fn drops_style_that_can_call_code() {
        for source in [
            r#"<div style="background: url(javascript:alert(1))">글</div>"#,
            r#"<div style="width: expression(alert(1))">글</div>"#,
            r#"<div style="@import 'evil.css'">글</div>"#,
            // 주석으로 감춰도 소용없다.
            r#"<div style="background: ur/**/l(//evil)">글</div>"#,
        ] {
            assert_eq!(
                HtmlNode::parse(source),
                vec![element(
                    HtmlTag::Division,
                    HtmlAttributes::default(),
                    vec![text("글")]
                )],
                "{source}"
            );
        }
    }

    // 열어 둔 태그가 새어 나가면 바깥 문서가 무너진다.
    #[test]
    fn closes_dangling_tags() {
        assert_eq!(
            HtmlNode::parse("<b>안 닫음"),
            vec![element(
                HtmlTag::Bold,
                HtmlAttributes::default(),
                vec![text("안 닫음")]
            )]
        );
        assert_eq!(
            HtmlNode::parse("</b>짝 없는 닫기"),
            vec![text("짝 없는 닫기")]
        );
    }

    // 겹친 태그는 안쪽부터만 닫힌다 — 짝이 어긋난 닫기는 버리고 우리가 닫는다.
    #[test]
    fn mismatched_close_is_dropped() {
        assert_eq!(
            HtmlNode::parse("<b><i>글</b></i>"),
            vec![element(
                HtmlTag::Bold,
                HtmlAttributes::default(),
                vec![element(
                    HtmlTag::Italic,
                    HtmlAttributes::default(),
                    vec![text("글")]
                )],
            )]
        );
    }

    #[test]
    fn stray_angle_brackets_stay_text() {
        assert_eq!(HtmlNode::parse("3 < 5"), vec![text("3 < 5")]);
        assert_eq!(
            HtmlNode::parse("<span>3 > 2</span>"),
            vec![element(
                HtmlTag::Span,
                HtmlAttributes::default(),
                vec![text("3 > 2")]
            )]
        );
    }
}
