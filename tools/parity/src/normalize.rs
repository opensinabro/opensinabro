//! HTML을 표현 차이가 제거된 "구조 지문"으로 정규화한다.
//!
//! the seed와 우리 백엔드는 골격(태그·중첩·style 시맨틱)이 같지만 표현이 다르다.
//! 클래스 어휘가 다르고(난독화 `t-m-VAGP` vs `wiki-table`), 색상 표기가 다르고
//! (`rgb(171,205,239)` vs `#abcdef`), style 선언 순서와 공백이 다르다.
//! 이런 차이를 걷어내야 남는 diff가 곧 의미론 차이다.

use std::fmt::Write as _;

/// 비교에서 제외하는 속성.
///
/// `class`는 제외하지 않는다 — 본문 클래스 어휘는 the seed와 우리가 같다
/// (`wiki-table`·`wiki-paragraph`·`toc-item` …). 난독화된 이름(`t-m-VAGP`)은
/// 페이지 셸의 것이지 마크업 산출물이 아니다.
///
/// - `data-v-*`: the seed 스킨의 Vue scoped 표식.
/// - `id`: 앵커 이름 규칙은 별도 과제라 지금은 비교하지 않는다.
/// - `data-dark-style`: the seed의 다크 색상 표현. 우리는 CSS 변수 + `.dark`로 처리해
///   표현 방식 자체가 다르다. 다크 파리티는 별도 과제다.
fn is_ignored_attribute(name: &str) -> bool {
    matches!(name, "id" | "data-dark-style") || name.starts_with("data-v-")
}

/// 문서 UI. 마크업 렌더링 산출물이 아니므로 통째로 들어낸다.
///
/// - `wiki-edit-section`: the seed의 문단 편집 링크.
/// - `wiki-categories`: 우리 백엔드가 붙이는 분류 푸터. the seed 본문 HTML에는 없다 —
///   분류 목록은 스킨이 그리는 것이라 본문 마크업에 들어가지 않는다.
///
/// 무시(known-differences)로는 부족하다 — 조각이 남아 있으면 이후 구조의 정렬이 밀려
/// 뒤따르는 모든 것이 차이로 잡히기 때문이다. 아예 없애야 양쪽이 다시 맞물린다.
fn is_dropped_subtree(class: &str) -> bool {
    class
        .split_whitespace()
        .any(|name| matches!(name, "wiki-edit-section" | "wiki-categories"))
}

/// 내용은 살리고 요소만 걷어내는 경우.
///
/// - 문단명 앵커 span(`<span id='개요'>개요</span>`): 우리는 `s-N` 앵커만 쓴다. 후속 과제라
///   지금은 껍데기만 벗기고 제목 텍스트는 비교한다.
/// - 구문 강조 토큰 span(`<span class='hljs-tag'>`…): the seed는 `#!syntax` 코드를
///   highlight.js로 토큰화하는데 우리는 통짜 텍스트로 낸다. 사용자 결정으로 무시한다 —
///   토큰 span만 벗기면 양쪽 다 코드 텍스트만 남아 다시 맞물린다.
fn is_unwrapped(name: &str, attributes: &[(String, String)]) -> bool {
    if name != "span" {
        return false;
    }
    let has_paragraph_anchor = attributes.iter().any(|(attribute, _)| attribute == "id")
        && !attributes
            .iter()
            .any(|(attribute, _)| attribute == "style" || attribute == "class");
    let is_highlight_token = attributes.iter().any(|(attribute, value)| {
        attribute == "class"
            && value
                .split_whitespace()
                .any(|name| name.starts_with("hljs-"))
    });
    has_paragraph_anchor || is_highlight_token
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragment {
    Open {
        name: String,
        attributes: Vec<(String, String)>,
    },
    Close {
        name: String,
    },
    Text(String),
}

impl Fragment {
    pub fn render(&self) -> String {
        match self {
            Fragment::Open { name, attributes } => {
                let mut output = format!("<{name}");
                for (attribute_name, value) in attributes {
                    write!(output, " {attribute_name}={value:?}").unwrap();
                }
                output.push('>');
                output
            }
            Fragment::Close { name } => format!("</{name}>"),
            Fragment::Text(text) => format!("{text:?}"),
        }
    }
}

/// 내용이 없는(자체 종료) HTML 요소.
fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "br" | "img" | "hr" | "input" | "meta" | "link" | "source"
    )
}

pub fn normalize(html: &str) -> Vec<Fragment> {
    let mut fragments = Vec::new();
    let bytes = html.as_bytes();
    let mut position = 0;
    let mut pending_text = String::new();
    let mut unwrapped_depth = 0usize;

    while position < bytes.len() {
        if bytes[position] == b'<' {
            let Some(end) = find_tag_end(html, position) else {
                break;
            };
            let inner = &html[position + 1..end];
            position = end + 1;
            if inner.starts_with('!') {
                continue; // 주석·doctype
            }
            if let Some(name) = inner.strip_prefix('/') {
                let name = name.trim().to_ascii_lowercase();
                // 걷어낸 요소는 텍스트를 끊지 않는다. 앞뒤 텍스트가 이어져야
                // 상대편의 통짜 텍스트와 맞물린다.
                if name == "span" && unwrapped_depth > 0 {
                    unwrapped_depth -= 1;
                    continue;
                }
                flush_text(&mut pending_text, &mut fragments);
                if !is_void_element(&name) {
                    fragments.push(Fragment::Close { name });
                }
                continue;
            }
            let inner = inner.strip_suffix('/').unwrap_or(inner);
            let (name, rest) = split_tag_name(inner);
            let name = name.to_ascii_lowercase();
            let raw_attributes = parse_attributes(rest);
            let void = is_void_element(&name);

            let class = raw_attributes
                .iter()
                .find(|(attribute, _)| attribute == "class")
                .map(|(_, value)| value.as_str())
                .unwrap_or_default();
            if !void && is_dropped_subtree(class) {
                position = skip_subtree(html, position, &name);
                continue;
            }
            if is_unwrapped(&name, &raw_attributes) {
                unwrapped_depth += 1;
                continue;
            }

            flush_text(&mut pending_text, &mut fragments);
            let attributes = normalize_attributes(raw_attributes);
            fragments.push(Fragment::Open {
                name: name.clone(),
                attributes,
            });
            if void {
                fragments.push(Fragment::Close { name });
            }
            continue;
        }
        let character_end = next_character_end(html, position);
        pending_text.push_str(&html[position..character_end]);
        position = character_end;
    }
    flush_text(&mut pending_text, &mut fragments);
    fragments
}

fn flush_text(pending: &mut String, fragments: &mut Vec<Fragment>) {
    let text = collapse_whitespace(&decode_entities(pending));
    pending.clear();
    if !text.is_empty() {
        fragments.push(Fragment::Text(text));
    }
}

fn next_character_end(html: &str, position: usize) -> usize {
    html[position..]
        .chars()
        .next()
        .map(|character| position + character.len_utf8())
        .unwrap_or(position + 1)
}

/// 태그 끝 `>`를 찾는다. 속성값 따옴표 안의 `>`는 건너뛴다.
fn find_tag_end(html: &str, start: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut position = start + 1;
    let mut quote = None;
    while position < bytes.len() {
        let byte = bytes[position];
        match (quote, byte) {
            (None, b'"') | (None, b'\'') => quote = Some(byte),
            (Some(open), _) if byte == open => quote = None,
            (None, b'>') => return Some(position),
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

/// 여는 태그 **바로 뒤**에서 시작해 짝이 맞는 닫는 태그 다음까지 건너뛴다.
/// 여는 태그는 호출부가 이미 소비했으므로 깊이 1에서 출발한다.
fn skip_subtree(html: &str, after_open: usize, name: &str) -> usize {
    let mut position = after_open;
    let mut depth = 1usize;
    while position < html.len() {
        let Some(next) = html[position..].find('<').map(|index| position + index) else {
            return html.len();
        };
        let Some(end) = find_tag_end(html, next) else {
            return html.len();
        };
        let inner = &html[next + 1..end];
        if let Some(close) = inner.strip_prefix('/') {
            if close.trim().eq_ignore_ascii_case(name) {
                depth -= 1;
                if depth == 0 {
                    return end + 1;
                }
            }
        } else if !inner.starts_with('!') && !inner.ends_with('/') {
            let (tag_name, _) = split_tag_name(inner);
            if tag_name.eq_ignore_ascii_case(name) {
                depth += 1;
            }
        }
        position = end + 1;
    }
    html.len()
}

fn normalize_attributes(raw: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut attributes: Vec<(String, String)> = raw
        .into_iter()
        .filter(|(name, _)| !is_ignored_attribute(name))
        .map(|(name, value)| {
            let value = if name == "style" {
                normalize_style(&value)
            } else {
                decode_entities(value.trim())
            };
            (name, value)
        })
        .filter(|(name, value)| !(name == "style" && value.is_empty()))
        .collect();
    attributes.sort();
    attributes
}

fn parse_attributes(rest: &str) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    let bytes = rest.as_bytes();
    let mut position = 0;
    while position < bytes.len() {
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        let name_start = position;
        while position < bytes.len()
            && !bytes[position].is_ascii_whitespace()
            && bytes[position] != b'='
        {
            position += 1;
        }
        if name_start == position {
            break;
        }
        let name = rest[name_start..position].to_ascii_lowercase();
        while position < bytes.len() && bytes[position].is_ascii_whitespace() {
            position += 1;
        }
        if position >= bytes.len() || bytes[position] != b'=' {
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
            let value = rest[start..position].to_string();
            position += 1;
            value
        } else {
            let start = position;
            while position < bytes.len() && !bytes[position].is_ascii_whitespace() {
                position += 1;
            }
            rest[start..position].to_string()
        };
        attributes.push((name, value));
    }
    attributes
}

/// style 선언을 파싱해 값 표기를 통일하고 이름순으로 정렬한다.
fn normalize_style(style: &str) -> String {
    let mut declarations: Vec<String> = decode_entities(style)
        .split(';')
        .filter_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            let property = property.trim().to_ascii_lowercase();
            let value = normalize_style_value(value.trim());
            (!property.is_empty() && !value.is_empty()).then(|| format!("{property}:{value}"))
        })
        .collect();
    declarations.sort();
    declarations.join(";")
}

fn normalize_style_value(value: &str) -> String {
    let collapsed = collapse_whitespace(value);
    let lowered = collapsed.to_ascii_lowercase();
    if let Some(color) = normalize_color(&lowered) {
        return color;
    }
    // `2px solid #ff0000`처럼 색이 섞인 복합 값은 토큰별로 정규화한다.
    lowered
        .split(' ')
        .map(|token| normalize_color(token).unwrap_or_else(|| token.to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 색상 표기를 6자리 소문자 hex로 통일한다. `#fff`, `rgb(255,255,255)`, `white` → `#ffffff`.
fn normalize_color(value: &str) -> Option<String> {
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 3 && hex.chars().all(|character| character.is_ascii_hexdigit()) {
            let mut expanded = String::from("#");
            for character in hex.chars() {
                expanded.push(character);
                expanded.push(character);
            }
            return Some(expanded);
        }
        if hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit()) {
            return Some(format!("#{hex}"));
        }
        return None;
    }
    if let Some(arguments) = value
        .strip_prefix("rgb(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let channels: Vec<u32> = arguments
            .split(',')
            .filter_map(|channel| channel.trim().parse().ok())
            .collect();
        if channels.len() == 3 && channels.iter().all(|channel| *channel <= 255) {
            return Some(format!(
                "#{:02x}{:02x}{:02x}",
                channels[0], channels[1], channels[2]
            ));
        }
        return None;
    }
    named_color(value).map(str::to_string)
}

/// 표·텍스트 색상에 흔히 쓰이는 CSS 색상 이름만 다룬다.
/// CSS 색상 이름 → hex. the seed는 이름을 hex로 바꿔 내므로(`red` → `#ff0000`) 표기를 맞춘다.
fn named_color(name: &str) -> Option<&'static str> {
    Some(match name {
        "aliceblue" => "#f0f8ff",
        "antiquewhite" => "#faebd7",
        "aqua" => "#00ffff",
        "aquamarine" => "#7fffd4",
        "azure" => "#f0ffff",
        "beige" => "#f5f5dc",
        "bisque" => "#ffe4c4",
        "black" => "#000000",
        "blanchedalmond" => "#ffebcd",
        "blue" => "#0000ff",
        "blueviolet" => "#8a2be2",
        "brown" => "#a52a2a",
        "burlywood" => "#deb887",
        "cadetblue" => "#5f9ea0",
        "chartreuse" => "#7fff00",
        "chocolate" => "#d2691e",
        "coral" => "#ff7f50",
        "cornflowerblue" => "#6495ed",
        "cornsilk" => "#fff8dc",
        "crimson" => "#dc143c",
        "cyan" => "#00ffff",
        "darkblue" => "#00008b",
        "darkcyan" => "#008b8b",
        "darkgoldenrod" => "#b8860b",
        "darkgray" => "#a9a9a9",
        "darkgreen" => "#006400",
        "darkgrey" => "#a9a9a9",
        "darkkhaki" => "#bdb76b",
        "darkmagenta" => "#8b008b",
        "darkolivegreen" => "#556b2f",
        "darkorange" => "#ff8c00",
        "darkorchid" => "#9932cc",
        "darkred" => "#8b0000",
        "darksalmon" => "#e9967a",
        "darkseagreen" => "#8fbc8f",
        "darkslateblue" => "#483d8b",
        "darkslategray" => "#2f4f4f",
        "darkslategrey" => "#2f4f4f",
        "darkturquoise" => "#00ced1",
        "darkviolet" => "#9400d3",
        "deeppink" => "#ff1493",
        "deepskyblue" => "#00bfff",
        "dimgray" => "#696969",
        "dimgrey" => "#696969",
        "dodgerblue" => "#1e90ff",
        "firebrick" => "#b22222",
        "floralwhite" => "#fffaf0",
        "forestgreen" => "#228b22",
        "fuchsia" => "#ff00ff",
        "gainsboro" => "#dcdcdc",
        "ghostwhite" => "#f8f8ff",
        "gold" => "#ffd700",
        "goldenrod" => "#daa520",
        "gray" => "#808080",
        "green" => "#008000",
        "greenyellow" => "#adff2f",
        "grey" => "#808080",
        "honeydew" => "#f0fff0",
        "hotpink" => "#ff69b4",
        "indianred" => "#cd5c5c",
        "indigo" => "#4b0082",
        "ivory" => "#fffff0",
        "khaki" => "#f0e68c",
        "lavender" => "#e6e6fa",
        "lavenderblush" => "#fff0f5",
        "lawngreen" => "#7cfc00",
        "lemonchiffon" => "#fffacd",
        "lightblue" => "#add8e6",
        "lightcoral" => "#f08080",
        "lightcyan" => "#e0ffff",
        "lightgoldenrodyellow" => "#fafad2",
        "lightgray" => "#d3d3d3",
        "lightgreen" => "#90ee90",
        "lightgrey" => "#d3d3d3",
        "lightpink" => "#ffb6c1",
        "lightsalmon" => "#ffa07a",
        "lightseagreen" => "#20b2aa",
        "lightskyblue" => "#87cefa",
        "lightslategray" => "#778899",
        "lightslategrey" => "#778899",
        "lightsteelblue" => "#b0c4de",
        "lightyellow" => "#ffffe0",
        "lime" => "#00ff00",
        "limegreen" => "#32cd32",
        "linen" => "#faf0e6",
        "magenta" => "#ff00ff",
        "maroon" => "#800000",
        "mediumaquamarine" => "#66cdaa",
        "mediumblue" => "#0000cd",
        "mediumorchid" => "#ba55d3",
        "mediumpurple" => "#9370db",
        "mediumseagreen" => "#3cb371",
        "mediumslateblue" => "#7b68ee",
        "mediumspringgreen" => "#00fa9a",
        "mediumturquoise" => "#48d1cc",
        "mediumvioletred" => "#c71585",
        "midnightblue" => "#191970",
        "mintcream" => "#f5fffa",
        "mistyrose" => "#ffe4e1",
        "moccasin" => "#ffe4b5",
        "navajowhite" => "#ffdead",
        "navy" => "#000080",
        "oldlace" => "#fdf5e6",
        "olive" => "#808000",
        "olivedrab" => "#6b8e23",
        "orange" => "#ffa500",
        "orangered" => "#ff4500",
        "orchid" => "#da70d6",
        "palegoldenrod" => "#eee8aa",
        "palegreen" => "#98fb98",
        "paleturquoise" => "#afeeee",
        "palevioletred" => "#db7093",
        "papayawhip" => "#ffefd5",
        "peachpuff" => "#ffdab9",
        "peru" => "#cd853f",
        "pink" => "#ffc0cb",
        "plum" => "#dda0dd",
        "powderblue" => "#b0e0e6",
        "purple" => "#800080",
        "rebeccapurple" => "#663399",
        "red" => "#ff0000",
        "rosybrown" => "#bc8f8f",
        "royalblue" => "#4169e1",
        "saddlebrown" => "#8b4513",
        "salmon" => "#fa8072",
        "sandybrown" => "#f4a460",
        "seagreen" => "#2e8b57",
        "seashell" => "#fff5ee",
        "sienna" => "#a0522d",
        "silver" => "#c0c0c0",
        "skyblue" => "#87ceeb",
        "slateblue" => "#6a5acd",
        "slategray" => "#708090",
        "slategrey" => "#708090",
        "snow" => "#fffafa",
        "springgreen" => "#00ff7f",
        "steelblue" => "#4682b4",
        "tan" => "#d2b48c",
        "teal" => "#008080",
        "thistle" => "#d8bfd8",
        "tomato" => "#ff6347",
        "turquoise" => "#40e0d0",
        "violet" => "#ee82ee",
        "wheat" => "#f5deb3",
        "white" => "#ffffff",
        "whitesmoke" => "#f5f5f5",
        "yellow" => "#ffff00",
        "yellowgreen" => "#9acd32",
        _ => return None,
    })
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        output.push_str(&rest[..index]);
        rest = &rest[index..];
        // 엔티티는 짧다. 다만 잘라 보는 구간이 UTF-8 경계를 밟지 않도록 문자 단위로 센다.
        let window: usize = rest
            .char_indices()
            .take(12)
            .last()
            .map(|(index, character)| index + character.len_utf8())
            .unwrap_or(rest.len());
        let Some(end) = rest[..window].find(';') else {
            output.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" | "#x27" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| {
                    entity
                        .strip_prefix('#')
                        .and_then(|digits| digits.parse().ok())
                })
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => {
                output.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                output.push('&');
                rest = &rest[1..];
            }
        }
    }
    output.push_str(rest);
    output
}
