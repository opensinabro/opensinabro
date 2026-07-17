//! `{{{#!html}}}` 원문을 안전한 부분집합으로 걸러 낸다.
//!
//! 나무마크는 위키 사용자가 쓰는 입력이고 `#!html`은 그 입력이 원시 HTML로 나가는
//! 유일한 통로다. 그래서 **화이트리스트**로 간다 — 아는 것만 통과시키고 나머지는
//! 버린다. 모르는 태그·속성이 생겼을 때 통과되는 쪽이 아니라 막히는 쪽이어야 한다.
//!
//! 나무위키는 `#!html`을 원시 HTML로 렌더한다(`{{{#!html <span style="…">글</span>}}}` →
//! `<span style="background-color:#999">글</span>`, `&nbsp;` → U+00A0). 다만 나무위키가
//! 정확히 무엇까지 허용하는지는 문서에 위험한 예제가 없어 렌더 대조로 확정할 수 없다.
//! 문법 도움말도 `#!html`을 "비권장, 지원 종료 가능"이라 서술한다. 그래서 허용 범위는
//! 도움말이 실제로 보여주는 표현(텍스트 서식, `<span class>`, `<div style>`)으로 좁힌다.

/// 통과시키는 태그. 텍스트 서식과 문단 구조에 더해 `<video>`까지다(렌더확정: `틀:video`의
/// `{{{#!html <video src="…" …></video>}}}`가 the seed에서 실제 `<video>`로 나온다).
/// 스크립트·삽입·폼은 없다.
const ALLOWED_TAGS: [&str; 17] = [
    "a", "b", "i", "u", "s", "strong", "em", "sub", "sup", "br", "span", "div", "code", "small",
    "big", "wbr", "video",
];

/// 통과시키는 속성. `id`는 문서 앵커(`s-1`, `fn-1`)와 충돌할 수 있어 받지 않고,
/// `src`는 나무마크의 이미지 문법이 대신하므로 열지 않는다.
///
/// `href`는 연다 — 나무위키가 그렇게 하고(렌더확정: `틀:문서 가져옴/나무위키`의
/// `{{{#!html <a href="https://namu.wiki/w/…">…</a>}}}`가 실제로 링크로 나온다),
/// [`safe_link`]가 나무마크 외부 링크와 같은 수준으로 깎아 내보낸다.
/// `src`·`width`·`height`·`controls`는 `<video>` 전용이다(아래 [`safe_attributes`]에서
/// 태그를 확인한다). 이미지의 `src`는 여전히 나무마크 문법이 대신한다.
const ALLOWED_ATTRIBUTES: [&str; 7] = [
    "class", "href", "style", "src", "width", "height", "controls",
];

/// 내용이 없는 태그.
const VOID_TAGS: [&str; 2] = ["br", "wbr"];

/// 허용 목록에 없는 태그·속성은 버리고, 텍스트는 이스케이프해 되살린다.
///
/// 태그를 버릴 때 그 안의 텍스트는 남긴다(껍데기만 벗긴다) — 사용자가 쓴 글이
/// 통째로 사라지는 편보다 낫다. 다만 `<script>`·`<style>`처럼 **내용 자체가 코드**인
/// 태그는 내용까지 버린다.
pub(crate) fn sanitize(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut open_tags: Vec<String> = Vec::new();
    let mut rest = source;

    while let Some(index) = rest.find('<') {
        push_text(&rest[..index], &mut output);
        rest = &rest[index..];

        let Some(end) = find_tag_end(rest) else {
            // 닫히지 않은 `<`는 글자일 뿐이다.
            push_text(rest, &mut output);
            return output;
        };
        let inner = &rest[1..end];
        rest = &rest[end + 1..];

        if let Some(name) = inner.strip_prefix('/') {
            let name = name.trim().to_ascii_lowercase();
            // 우리가 연 적이 있는 태그만 닫는다. 짝이 안 맞는 닫기는 버린다.
            if open_tags.last() == Some(&name) {
                open_tags.pop();
                output.push_str("</");
                output.push_str(&name);
                output.push('>');
            }
            continue;
        }
        if inner.starts_with('!') {
            continue; // 주석·doctype
        }

        let inner = inner.strip_suffix('/').unwrap_or(inner);
        let (name, attributes) = split_tag_name(inner);
        let name = name.to_ascii_lowercase();

        if is_code_bearing(&name) {
            rest = skip_until_close(rest, &name);
            continue;
        }
        if !ALLOWED_TAGS.contains(&name.as_str()) {
            continue; // 껍데기만 버리고 내용은 계속 읽는다
        }
        let safe = safe_attributes(&name, attributes);
        // 갈 곳이 없는 링크는 링크가 아니다 — 껍데기를 벗기고 글만 남긴다.
        if name == "a" && !safe.iter().any(|(attribute, _)| attribute == "href") {
            continue;
        }

        output.push('<');
        output.push_str(&name);
        for (attribute, value) in safe {
            output.push(' ');
            output.push_str(&attribute);
            output.push_str("=\"");
            push_attribute_value(&value, &mut output);
            output.push('"');
        }
        output.push('>');
        if !VOID_TAGS.contains(&name.as_str()) {
            open_tags.push(name);
        }
    }
    push_text(rest, &mut output);

    // 열어 둔 태그는 우리가 닫는다 — 새어 나가면 바깥 문서 구조가 무너진다.
    while let Some(name) = open_tags.pop() {
        output.push_str("</");
        output.push_str(&name);
        output.push('>');
    }
    output
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

fn safe_attributes(tag: &str, source: &str) -> Vec<(String, String)> {
    let mut attributes: Vec<(String, String)> = parse_attributes(source)
        .into_iter()
        .filter(|(name, _)| ALLOWED_ATTRIBUTES.contains(&name.as_str()))
        .filter_map(|(name, value)| match name.as_str() {
            "style" => safe_style(&value).map(|value| (name, value)),
            // `href`는 링크에만 뜻이 있다.
            "href" if tag != "a" => None,
            "href" => safe_link(&value).map(|value| (name, value)),
            // 미디어 속성은 `<video>` 전용이고, `src`는 링크와 같은 수준으로 깎는다.
            "src" | "width" | "height" | "controls" if tag != "video" => None,
            "src" => safe_link(&value).map(|value| (name, value)),
            _ => Some((name, value)),
        })
        .collect();
    // 위키 입력이 만든 링크는 나무마크 외부 링크와 똑같이 꾸며 내보낸다 — 새 창으로 열고,
    // 검색 엔진이 따라가지 않으며, 연 문서의 창 객체를 넘기지 않는다.
    if tag == "a" && attributes.iter().any(|(name, _)| name == "href") {
        attributes.retain(|(name, _)| name != "class");
        attributes.push(("target".to_string(), "_blank".to_string()));
        attributes.push(("rel".to_string(), "nofollow noopener ugc".to_string()));
        attributes.push(("class".to_string(), "wiki-link-external".to_string()));
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

/// style 값에서 코드를 부를 수 있는 표현을 막는다.
///
/// `url(...)`은 외부 요청과 `javascript:`를, `expression(...)`은 옛 IE의 스크립트를
/// 부른다. 하나라도 있으면 그 style 전체를 버린다 — 부분만 지우면 남은 값이
/// 무엇이 될지 장담할 수 없다.
fn safe_style(value: &str) -> Option<String> {
    let lowered = value.to_ascii_lowercase();
    let dangerous = ["url(", "expression(", "javascript:", "@import", "behavior:"];
    if dangerous.iter().any(|pattern| lowered.contains(pattern)) {
        return None;
    }
    Some(value.to_string())
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

/// 텍스트의 문자 참조를 실제 글자로 바꾸고, 태그가 될 수 있는 글자만 막는다.
///
/// 나무위키는 `#!html`의 엔티티를 글자로 바꿔 내보낸다(`&nbsp;` → U+00A0,
/// `&#8203` → U+200B). 세미콜론이 빠져도 브라우저처럼 받아 준다 — 실제 문서가
/// `{{{#!html &#8203}}}`처럼 쓴다.
fn push_text(text: &str, output: &mut String) {
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        push_escaped(&rest[..index], output);
        rest = &rest[index..];
        match decode_entity(rest) {
            Some((character, length)) => {
                // 디코딩한 글자도 태그가 되어선 안 된다(`&lt;script&gt;` 같은 우회).
                push_escaped(&character.to_string(), output);
                rest = &rest[length..];
            }
            None => {
                output.push('&');
                rest = &rest[1..];
            }
        }
    }
    push_escaped(rest, output);
}

fn push_escaped(text: &str, output: &mut String) {
    for character in text.chars() {
        match character {
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            // 디코딩으로 되살아난 `&`는 다시 엔티티를 이루지 못하게 이스케이프한다
            // (렌더확정: `&amp;nbsp;`는 the seed에서 `&amp;nbsp;` 그대로 — `&amp;`가 U+00A0가
            // 아니라 `&` 글자로 남는다).
            '&' => output.push_str("&amp;"),
            _ => output.push(character),
        }
    }
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

fn push_attribute_value(value: &str, output: &mut String) {
    for character in value.chars() {
        match character {
            '"' => output.push_str("&quot;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_allowed_markup() {
        assert_eq!(
            sanitize(r#"<span style="background-color: #999">배경색 적용</span>"#),
            r#"<span style="background-color: #999">배경색 적용</span>"#
        );
        assert_eq!(sanitize("<b>굵게</b>"), "<b>굵게</b>");
        assert_eq!(sanitize("줄<br>바꿈"), "줄<br>바꿈");
    }

    // 나무위키는 엔티티를 실제 글자로 바꿔 내보낸다.
    #[test]
    fn decodes_entities() {
        assert_eq!(sanitize("&nbsp;"), "\u{a0}");
        assert_eq!(sanitize("&ZeroWidthSpace;"), "\u{200b}");
        assert_eq!(sanitize("&#8203;"), "\u{200b}");
        // 실제 문서는 세미콜론을 빼먹기도 한다 — 브라우저처럼 받아 준다.
        assert_eq!(sanitize("&#8203"), "\u{200b}");
        assert_eq!(sanitize("&#x200b;"), "\u{200b}");
    }

    // 디코딩한 글자가 태그가 되어선 안 된다.
    #[test]
    fn decoded_entities_cannot_become_tags() {
        assert_eq!(
            sanitize("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(sanitize("&#60;script&#62;"), "&lt;script&gt;");
    }

    #[test]
    fn keeps_unknown_ampersand_as_is() {
        assert_eq!(sanitize("A & B"), "A & B");
        assert_eq!(sanitize("&unknown;"), "&unknown;");
    }

    #[test]
    fn drops_script_with_its_content() {
        assert_eq!(sanitize("앞<script>alert(1)</script>뒤"), "앞뒤");
        assert_eq!(sanitize("<style>body{display:none}</style>"), "");
        assert_eq!(sanitize("<iframe src='//evil'></iframe>"), "");
    }

    // 허용하지 않는 태그는 껍데기만 벗기고 글은 남긴다.
    #[test]
    fn unwraps_unknown_tags() {
        assert_eq!(sanitize("<marquee>글</marquee>"), "글");
        assert_eq!(sanitize("<img src='//evil'>"), "");
    }

    // 위키 입력이 만든 링크도 나무마크 외부 링크와 같은 수준으로 나간다.
    #[test]
    fn keeps_web_links_and_hardens_them() {
        assert_eq!(
            sanitize("<a href='https://namu.wiki/w/X'>글</a>"),
            "<a href=\"https://namu.wiki/w/X\" target=\"_blank\" \
             rel=\"nofollow noopener ugc\" class=\"wiki-link-external\">글</a>"
        );
    }

    // 갈 곳이 없거나 코드를 부르는 링크는 껍데기를 벗기고 글만 남긴다.
    #[test]
    fn unwraps_links_that_are_not_web_addresses() {
        assert_eq!(sanitize("<a href='javascript:alert(1)'>글</a>"), "글");
        assert_eq!(sanitize("<a href='data:text/html,<script>'>글</a>"), "글");
        assert_eq!(sanitize("<a href='//evil'>글</a>"), "글");
        assert_eq!(sanitize("<a href='/w/문서'>글</a>"), "글");
        assert_eq!(sanitize("<a>글</a>"), "글");
        // 스킴을 대소문자로 감춰도 소용없다.
        assert_eq!(sanitize("<a href='JaVaScRiPt:alert(1)'>글</a>"), "글");
    }

    // 위키 입력이 링크 클래스를 제 마음대로 정하지 못한다.
    #[test]
    fn link_class_is_ours() {
        assert_eq!(
            sanitize("<a class='evil' href='https://x.test/'>글</a>"),
            "<a href=\"https://x.test/\" target=\"_blank\" \
             rel=\"nofollow noopener ugc\" class=\"wiki-link-external\">글</a>"
        );
    }

    #[test]
    fn drops_event_handlers_and_unknown_attributes() {
        assert_eq!(
            sanitize(r#"<span onerror="alert(1)" onclick="x()">글</span>"#),
            "<span>글</span>"
        );
        assert_eq!(sanitize(r#"<div id="s-1">글</div>"#), "<div>글</div>");
    }

    #[test]
    fn drops_style_that_can_call_code() {
        assert_eq!(
            sanitize(r#"<div style="background: url(javascript:alert(1))">글</div>"#),
            "<div>글</div>"
        );
        assert_eq!(
            sanitize(r#"<div style="width: expression(alert(1))">글</div>"#),
            "<div>글</div>"
        );
        assert_eq!(
            sanitize(r#"<div style="@import 'evil.css'">글</div>"#),
            "<div>글</div>"
        );
    }

    // 열어 둔 태그가 새어 나가면 바깥 문서가 무너진다.
    #[test]
    fn closes_dangling_tags() {
        assert_eq!(sanitize("<b>안 닫음"), "<b>안 닫음</b>");
        assert_eq!(sanitize("</b>짝 없는 닫기"), "짝 없는 닫기");
    }

    #[test]
    fn escapes_stray_angle_brackets() {
        assert_eq!(sanitize("3 < 5"), "3 &lt; 5");
        assert_eq!(sanitize("<span>3 > 2</span>"), "<span>3 &gt; 2</span>");
    }

    #[test]
    fn escapes_quotes_in_attribute_values() {
        assert_eq!(
            sanitize(r#"<span class='a"><script>alert(1)</script>'>글</span>"#),
            r#"<span class="a&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;">글</span>"#
        );
    }
}
