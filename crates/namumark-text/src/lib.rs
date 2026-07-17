//! 나무마크 표기의 문자열 수준 판정 유틸리티.
//!
//! 구문 크레이트(경계 결정)와 파서 크레이트(의미 해석)가 공유한다.
//! 의미 모델에 의존하지 않도록 판정 결과는 이 크레이트의 자체 어휘
//! ([`ListMarkerKind`], [`CellOption`] 등)로 표현하고, 의미 모델로의 매핑은
//! 소비 크레이트가 담당한다.

use std::ops::Range;

pub fn find_matching_braces(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"{{{") {
            depth += 1;
            index += 3;
        } else if bytes[index..].starts_with(b"}}}") {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    None
}

pub fn find_matching_bracket(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// `[[…]]`의 닫는 자리. `\[`·`\]`는 글자라 짝으로 세지 않는다
/// (렌더확정: `[[[\X\]]]`를 the seed는 제목이 `[X]`인 문서로 보낸다).
pub fn find_matching_double_bracket(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
        } else if bytes[index..].starts_with(b"[[") {
            depth += 1;
            index += 2;
        } else if bytes[index..].starts_with(b"]]") {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    None
}

/// 이 줄 끝에서 `{{{` 그룹이 열린 채 남는가 — 남으면 다음 줄까지 이어진다.
///
/// 닫히지 않는 `{{{`는 그룹이 아니라 글자다. 나무위키는 `{` 하나를 글자로 흘리고
/// 다음 자리에서 다시 여는데(렌더확정: `{{{{{{-5 {{{-5 -10단계}}}}}}` →
/// `{` + 리터럴 `{{-5 {{{-5 -10단계}}}`), 이 복구까지 마친 뒤에도 짝을 못 찾은
/// 그룹이 있어야 비로소 열린 것이다.
pub fn has_open_group(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    // 짝을 못 찾아 글자로 흘려보낸 `{{{`의 자리. 바로 뒤(한두 칸 안)에서 그룹이 열리면
    // 그 브레이스들이 거기에 쓰인 것이므로 취소된다.
    let mut unmatched: Option<usize> = None;
    while index < bytes.len() {
        if !bytes[index..].starts_with(b"{{{") {
            index += 1;
            continue;
        }
        match find_matching_braces(&text[index..]) {
            Some(close) => {
                if unmatched.is_some_and(|start| index - start <= 2) {
                    unmatched = None;
                }
                index += close + 3;
            }
            None => {
                unmatched = unmatched.or(Some(index));
                index += 1;
            }
        }
    }
    unmatched.is_some()
}

pub fn brace_delta(line: &str) -> i32 {
    let bytes = line.as_bytes();
    let mut delta = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"{{{") {
            delta += 1;
            index += 3;
        } else if bytes[index..].starts_with(b"}}}") {
            delta -= 1;
            index += 3;
        } else {
            index += 1;
        }
    }
    delta
}

pub fn parse_size_marker(content: &str) -> Option<(i8, &str)> {
    let (sign, rest) = if let Some(rest) = content.strip_prefix('+') {
        (1i8, rest)
    } else if let Some(rest) = content.strip_prefix('-') {
        (-1i8, rest)
    } else {
        return None;
    };
    let digit = rest.chars().next()?;
    if !('1'..='5').contains(&digit) {
        return None;
    }
    let level = sign * (digit as u8 - b'0') as i8;
    let rest = &rest[1..];
    if rest.is_empty() {
        return Some((level, ""));
    }
    rest.strip_prefix(' ').map(|rest| (level, rest))
}

/// 한 줄 색상 그룹 `{{{#색상 내용}}}`의 내용부를 가른다.
///
/// 색상 표기 뒤에는 내용을 가르는 공백이 있어야 한다. `{{{#212529}}}`처럼 없으면
/// 색상이 아니라 그냥 리터럴이다(렌더확정: the seed는 `<code>#212529</code>`로 낸다).
pub fn parse_color_marker(content: &str) -> Option<(String, Option<String>, &str)> {
    let (specification, rest) = content.split_once(' ')?;
    let (color, dark_color) = parse_color_specification(specification)?;
    Some((color, dark_color, rest))
}

/// 여러 줄 색상 그룹의 헤더 `{{{#색상`에서 색상만 읽는다. 내용은 다음 줄부터라
/// 개행이 구분자이고, 헤더에 공백이 있으면 그 뒤는 내용의 첫 조각이다.
pub fn parse_color_specification(header: &str) -> Option<(String, Option<String>)> {
    if !header.starts_with('#') {
        return None;
    }
    let specification = header.split(' ').next()?;
    let (first, second) = match specification.split_once(',') {
        Some((first, second)) => (first, Some(second)),
        None => (specification, None),
    };
    let color = parse_color(first)?;
    let dark_color = match second {
        Some(second) => Some(parse_color(second)?),
        None => None,
    };
    Some((color, dark_color))
}

/// CSS가 정의한 색상 이름. the seed는 이 목록에 있는 이름만 색상으로 받는다
/// (렌더확정: `{{{#redirect 목적지 문서}}}`는 색상이 아니라 리터럴이다).
const CSS_COLOR_NAMES: [&str; 148] = [
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

/// CSS가 정의한 색상 이름인가. 나무위키는 이 목록에 있는 이름만 색으로 받는다.
pub fn is_css_color_name(name: &str) -> bool {
    CSS_COLOR_NAMES
        .binary_search(&name.to_ascii_lowercase().as_str())
        .is_ok()
}

// 색상은 `#` 접두사로 표기한다. hex는 `#` 포함 그대로, 색상 이름은 `#`를 제거해 보관한다.
fn parse_color(source: &str) -> Option<String> {
    let body = source.strip_prefix('#')?;
    if matches!(body.len(), 3 | 6) && body.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(source.to_string())
    } else if is_css_color_name(body) {
        Some(body.to_string())
    } else {
        None
    }
}

/// 헤딩 라인의 콘텐츠 범위 (마커·구분 공백 제외). 수준·접기 여부는
/// 소비 측이 마커 텍스트에서 계산한다.
pub struct HeadingShape {
    pub content_start: usize,
    pub content_end: usize,
}

pub fn heading_shape(line: &str) -> Option<HeadingShape> {
    let level = line.bytes().take_while(|&byte| byte == b'=').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &line[level..];
    let (folded, rest) = match rest.strip_prefix('#') {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let rest = rest.strip_prefix(' ')?;
    let closing = if folded {
        format!("#{}", "=".repeat(level))
    } else {
        "=".repeat(level)
    };
    let content = rest.strip_suffix(closing.as_str())?.strip_suffix(' ')?;
    if content.is_empty() {
        return None;
    }
    let content_start = line.len() - rest.len();
    Some(HeadingShape {
        content_start,
        content_end: content_start + content.len(),
    })
}

pub fn is_horizontal_rule(line: &str) -> bool {
    (4..=9).contains(&line.len()) && line.bytes().all(|byte| byte == b'-')
}

pub fn parse_redirect(line: &str) -> Option<String> {
    let target = line
        .strip_prefix("#redirect ")
        .or_else(|| line.strip_prefix("#넘겨주기 "))?;
    Some(target.trim().to_string())
}

// ---- 리스트 ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMarkerKind {
    Unordered,
    Decimal,
    LowerAlphabet,
    UpperAlphabet,
    LowerRoman,
    UpperRoman,
}

pub struct ListMarker<'line> {
    pub kind: ListMarkerKind,
    /// `1.#42`처럼 번호를 재지정하는 경우에만 값이 있다.
    pub start_number: Option<u32>,
    pub content: &'line str,
}

/// 순서 리스트 마커는 항상 `1.` `a.` 등 리터럴이고 번호는 자동 증가한다.
/// 마커 뒤 공백은 선택이며, `1.#42`는 시작 번호를 재지정한다.
pub fn list_marker(line: &str) -> Option<ListMarker<'_>> {
    if let Some(rest) = line.strip_prefix('*') {
        return Some(ListMarker {
            kind: ListMarkerKind::Unordered,
            start_number: None,
            content: strip_single_space(rest),
        });
    }
    const ORDERED_MARKERS: [(&str, ListMarkerKind); 5] = [
        ("1.", ListMarkerKind::Decimal),
        ("a.", ListMarkerKind::LowerAlphabet),
        ("A.", ListMarkerKind::UpperAlphabet),
        ("i.", ListMarkerKind::LowerRoman),
        ("I.", ListMarkerKind::UpperRoman),
    ];
    for (marker, kind) in ORDERED_MARKERS {
        let Some(rest) = line.strip_prefix(marker) else {
            continue;
        };
        let (start_number, rest) = match rest.strip_prefix('#') {
            Some(after) => {
                let digits_end = after
                    .bytes()
                    .take_while(|byte| byte.is_ascii_digit())
                    .count();
                (after[..digits_end].parse().ok(), &after[digits_end..])
            }
            None => (None, rest),
        };
        return Some(ListMarker {
            kind,
            start_number,
            content: strip_single_space(rest),
        });
    }
    None
}

fn strip_single_space(rest: &str) -> &str {
    rest.strip_prefix(' ').unwrap_or(rest)
}

// ---- 링크 ----

pub fn strip_link_prefix<'target>(target: &'target str, prefixes: &[&str]) -> Option<&'target str> {
    for prefix in prefixes {
        if target.len() >= prefix.len()
            && target.is_char_boundary(prefix.len())
            && target[..prefix.len()].eq_ignore_ascii_case(prefix)
        {
            return Some(&target[prefix.len()..]);
        }
    }
    None
}

pub fn split_anchor(target: &str) -> (&str, Option<String>) {
    if strip_link_prefix(target, &["http://", "https://", "ftp://"]).is_some() {
        return (target, None);
    }
    // 마지막 `#`은 앵커 구분자다. 뒤가 비어 있어도 구분자 노릇은 하므로 대상에서
    // 떨어져 나간다(렌더확정: the seed는 `[[##]]`를 제목이 `#`인 문서로 보낸다).
    match last_unescaped(target, b'#') {
        Some(index) => (
            &target[..index],
            Some(target[index + 1..].to_string()).filter(|anchor| !anchor.is_empty()),
        ),
        None => (target, None),
    }
}

/// `[[대상|표시]]`의 본문을 대상과 표시부로 가른다. `\|`는 글자라 구분자가 아니다
/// (렌더확정: the seed는 `[[\|]]`를 제목이 `|`인 문서로 보낸다).
pub fn split_link_body(body: &str) -> (&str, Option<&str>) {
    match first_unescaped(body, b'|') {
        Some(index) => (&body[..index], Some(&body[index + 1..])),
        None => (body, None),
    }
}

/// `\`가 앞에 붙지 않은 첫 `needle`의 자리.
fn first_unescaped(text: &str, needle: u8) -> Option<usize> {
    unescaped_positions(text, needle).next()
}

fn last_unescaped(text: &str, needle: u8) -> Option<usize> {
    unescaped_positions(text, needle).last()
}

fn unescaped_positions(text: &str, needle: u8) -> impl Iterator<Item = usize> + '_ {
    let bytes = text.as_bytes();
    let mut index = 0;
    std::iter::from_fn(move || {
        while index < bytes.len() {
            let at = index;
            index += 1;
            match bytes[at] {
                b'\\' => index += 1,
                byte if byte == needle => return Some(at),
                _ => {}
            }
        }
        None
    })
}

/// `\X`를 `X`로 되돌린다. 링크 대상처럼 인라인 파서를 거치지 않는 문자열에 쓴다.
pub fn unescape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            if let Some(escaped) = characters.next() {
                output.push(escaped);
            }
        } else {
            output.push(character);
        }
    }
    output
}

// ---- 지시자 ----

pub fn is_table_start(line: &str) -> bool {
    line.starts_with("||") || (line.starts_with('|') && line[1..].contains('|'))
}

pub fn strip_directive<'line>(header: &'line str, directive: &str) -> Option<&'line str> {
    let rest = header.strip_prefix(directive)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.strip_prefix(' ').map(str::trim_start)
}

pub fn parse_wiki_style_attributes(source: &str) -> (Option<String>, Option<String>, &str) {
    let mut style: Option<String> = None;
    let mut dark_style: Option<String> = None;
    let mut rest = source;
    loop {
        rest = rest.trim_start();
        let (is_dark, value_source) = if let Some(after) = rest.strip_prefix("style=") {
            (false, after)
        } else if let Some(after) = rest.strip_prefix("dark-style=") {
            (true, after)
        } else {
            break;
        };
        let Some((value, remainder)) = parse_quoted(value_source) else {
            break;
        };
        let target = if is_dark { &mut dark_style } else { &mut style };
        match target {
            Some(existing) => existing.push_str(value),
            None => *target = Some(value.to_string()),
        }
        rest = remainder;
    }
    (style, dark_style, rest.trim())
}

fn parse_quoted(source: &str) -> Option<(&str, &str)> {
    let quote = source.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &source[1..];
    let end = rest.find(quote)?;
    Some((&rest[..end], &rest[end + 1..]))
}

// ---- 표 셀 ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPosition {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellOptionScope {
    Cell,
    Row,
    Column,
    Table,
}

/// `<...>` 옵션 토큰의 분류 결과. 의미 모델로의 매핑은 소비 측이 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellOption<'source> {
    Alignment(CellAlignment),
    ColumnSpan(u32),
    RowSpan {
        span: u32,
        vertical_position: Option<VerticalPosition>,
    },
    Flag {
        scope: CellOptionScope,
        name: &'static str,
    },
    Attribute {
        scope: CellOptionScope,
        name: &'static str,
        value: &'source str,
    },
    BackgroundColor(&'source str),
}

pub struct CellShape<'source> {
    /// 유효한 옵션 토큰들 (등장 순서 보존)
    pub options: Vec<CellOption<'source>>,
    /// `<옵션>` 나열이 끝나는 위치
    pub options_end: usize,
    /// 셀 텍스트 내 콘텐츠 범위 (옵션·정렬 공백 제외)
    pub content_start: usize,
    pub content_end: usize,
    /// 옵션으로 지정됐거나 공백 규칙으로 유도된 정렬. None이면 기본(왼쪽).
    pub alignment: Option<CellAlignment>,
}

/// 셀 원문(선행·후행 `||` 제외)에서 옵션·정렬·콘텐츠 경계를 판정한다.
pub fn cell_shape(source: &str) -> CellShape<'_> {
    let mut options = Vec::new();
    let mut rest = source;
    while let Some(after_open) = rest.strip_prefix('<') {
        let Some(close) = after_open.find('>') else {
            break;
        };
        let token = &after_open[..close];
        if token.is_empty() || token.contains('<') {
            break;
        }
        let Some(option) = classify_cell_option(token) else {
            break;
        };
        options.push(option);
        rest = &after_open[close + 1..];
    }
    let options_end = source.len() - rest.len();

    let mut alignment = options
        .iter()
        .filter_map(|option| match option {
            CellOption::Alignment(alignment) => Some(*alignment),
            _ => None,
        })
        .next_back();
    let explicit_alignment = alignment.is_some();

    let mut content = rest;
    if explicit_alignment {
        content = content.strip_prefix(' ').unwrap_or(content);
        content = content.strip_suffix(' ').unwrap_or(content);
    } else if let Some(stripped) = content.strip_prefix(' ') {
        if let Some(both) = stripped.strip_suffix(' ') {
            alignment = Some(CellAlignment::Center);
            content = both;
        } else {
            alignment = Some(CellAlignment::Right);
            content = stripped;
        }
    } else {
        content = content.strip_suffix(' ').unwrap_or(content);
    }
    let content_offset = content.as_ptr() as usize - source.as_ptr() as usize;
    CellShape {
        options,
        options_end,
        content_start: content_offset,
        content_end: content_offset + content.len(),
        alignment,
    }
}

fn classify_cell_option(token: &str) -> Option<CellOption<'_>> {
    match token {
        "(" => return Some(CellOption::Alignment(CellAlignment::Left)),
        ":" => return Some(CellOption::Alignment(CellAlignment::Center)),
        ")" => return Some(CellOption::Alignment(CellAlignment::Right)),
        "keepall" => {
            return Some(CellOption::Flag {
                scope: CellOptionScope::Cell,
                name: "keepall",
            });
        }
        "nopad" => {
            return Some(CellOption::Flag {
                scope: CellOptionScope::Cell,
                name: "nopad",
            });
        }
        "rowkeepall" => {
            return Some(CellOption::Flag {
                scope: CellOptionScope::Row,
                name: "keepall",
            });
        }
        "colkeepall" => {
            return Some(CellOption::Flag {
                scope: CellOptionScope::Column,
                name: "keepall",
            });
        }
        _ => {}
    }

    if let Some(number) = token.strip_prefix('-')
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(value) = number.parse::<u32>()
    {
        return Some(CellOption::ColumnSpan(value.max(1)));
    }

    let (vertical_position, rowspan_source) = if let Some(rest) = token.strip_prefix('^') {
        (Some(VerticalPosition::Top), rest)
    } else if let Some(rest) = token.strip_prefix('v') {
        (Some(VerticalPosition::Bottom), rest)
    } else {
        (None, token)
    };
    if let Some(number) = rowspan_source.strip_prefix('|')
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(value) = number.parse::<u32>()
    {
        return Some(CellOption::RowSpan {
            span: value.max(1),
            vertical_position,
        });
    }

    if let Some((name, value)) = token.split_once('=') {
        let normalized = name.replace(' ', "").to_ascii_lowercase();
        let (scope, canonical) = resolve_attribute_name(&normalized)?;
        return Some(CellOption::Attribute {
            scope,
            name: canonical,
            value,
        });
    }

    if is_bare_color(token) {
        return Some(CellOption::BackgroundColor(token));
    }

    None
}

fn resolve_attribute_name(name: &str) -> Option<(CellOptionScope, &'static str)> {
    const TABLE_NAMES: [&str; 8] = [
        "bgcolor",
        "width",
        "height",
        "align",
        "class",
        "textalign",
        "color",
        "bordercolor",
    ];
    const ROW_NAMES: [&str; 3] = ["bgcolor", "textalign", "color"];
    const COLUMN_NAMES: [&str; 3] = ["bgcolor", "color", "textalign"];
    const CELL_NAMES: [&str; 4] = ["bgcolor", "color", "width", "height"];

    fn canonical(names: &[&'static str], rest: &str) -> Option<&'static str> {
        names.iter().copied().find(|name| *name == rest)
    }

    if let Some(rest) = name.strip_prefix("table")
        && let Some(canonical) = canonical(&TABLE_NAMES, rest)
    {
        return Some((CellOptionScope::Table, canonical));
    }
    if let Some(rest) = name.strip_prefix("row")
        && let Some(canonical) = canonical(&ROW_NAMES, rest)
    {
        return Some((CellOptionScope::Row, canonical));
    }
    if let Some(rest) = name.strip_prefix("col")
        && let Some(canonical) = canonical(&COLUMN_NAMES, rest)
    {
        return Some((CellOptionScope::Column, canonical));
    }
    if let Some(canonical) = canonical(&CELL_NAMES, name) {
        return Some((CellOptionScope::Cell, canonical));
    }
    None
}

fn is_bare_color(token: &str) -> bool {
    if let Some(hex) = token.strip_prefix('#') {
        matches!(hex.len(), 3 | 6) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else {
        // 이름형 배경색은 실제 CSS 색상명만이다 — `<br>`·`<sup>` 같은 임의의 단어를
        // 색으로 오인하면 셀 옵션으로 먹혀 사라진다(렌더확정: `||<br>을 …`의 `<br>`은 글자).
        is_css_color_name(token)
    }
}

/// 행 원문을 (선행 `||` 쌍 수, 셀 텍스트 범위)로 분리한다.
/// 이 브레이스 런의 한두 칸 뒤에서 짝이 맞는 그룹이 열리는가.
pub fn opens_within_run(rest: &str) -> bool {
    (1..=2).any(|offset| {
        rest.as_bytes()[offset..].starts_with(b"{{{")
            && find_matching_braces(&rest[offset..]).is_some()
    })
}

pub fn split_cell_ranges(row_source: &str) -> Vec<(usize, std::ops::Range<usize>)> {
    let bytes = row_source.as_bytes();
    let mut cells = Vec::new();
    let mut span_pairs = pipe_run_length(bytes, 0) / 2;
    let mut position = span_pairs * 2;
    let mut cell_start = position;
    while position < bytes.len() {
        // `\|`는 글자라 셀 구분자가 아니다(렌더확정: `|| {{{\|| 표 문법 무효 \||}}} || … ||`가
        // the seed에서 두 셀이다).
        if bytes[position] == b'\\' {
            position += 2;
            continue;
        }
        // 짝이 맞는 그룹은 통째로 건너뛴다.
        if bytes[position..].starts_with(b"{{{") {
            match find_matching_braces(&row_source[position..]) {
                Some(close) => {
                    position += close + 3;
                    continue;
                }
                // 바로 옆에서 다시 열어 짝이 맞으면 이 `{`는 글자다(the seed의 복구 —
                // [`has_open_group`]). 그것도 아니면 이 그룹은 정말 열린 채라
                // 남은 줄을 통째로 머금는다 — 그 안의 `||`는 셀 구분자가 아니다.
                None if !opens_within_run(&row_source[position..]) => break,
                None => {
                    position += 1;
                    continue;
                }
            }
        }
        if bytes[position] == b'|' {
            let run = pipe_run_length(bytes, position);
            if run >= 2 {
                cells.push((span_pairs, cell_start..position));
                span_pairs = run / 2;
                position += span_pairs * 2;
                cell_start = position;
            } else {
                position += 1;
            }
        } else {
            position += 1;
        }
    }
    if cell_start < bytes.len() {
        let trailing_length = row_source[cell_start..].trim_end().len();
        if trailing_length > 0 {
            cells.push((span_pairs, cell_start..cell_start + trailing_length));
        }
    }
    cells
}

pub fn pipe_run_length(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .take_while(|&&byte| byte == b'|')
        .count()
}

pub fn is_row_complete(row_source: &str) -> bool {
    if has_open_group(row_source) {
        return false;
    }
    let trimmed = row_source.trim_end();
    let without_pipes = trimmed.trim_end_matches('|');
    if without_pipes.is_empty() {
        return trimmed.len() >= 4;
    }
    trimmed.ends_with("||")
}

/// 표 첫 줄의 `|캡션|` 범위.
pub fn caption_range(line: &str) -> Option<std::ops::Range<usize>> {
    if !line.starts_with('|') || line.starts_with("||") {
        return None;
    }
    let rest = &line[1..];
    let end = rest.find('|')?;
    Some(1..1 + end)
}

/// 틀 인자 표기의 모양. `@이름@` 또는 `@이름=기본값@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableShape {
    /// `@`를 포함한 표기 전체의 길이.
    pub length: usize,
    /// 표기 안에서 이름이 차지하는 범위.
    pub name: Range<usize>,
    /// 기본값 범위. `=`가 없으면 None이다.
    pub default: Option<Range<usize>>,
}

/// 문자열 맨 앞이 틀 인자 표기인지 본다.
///
/// 이름과 기본값에는 `@`와 줄바꿈이 올 수 없다. 이 규칙 덕에 본문의 평범한 `@`
/// (이메일 주소 등)는 인자로 오인되지 않는다.
pub fn variable_shape(source: &str) -> Option<VariableShape> {
    let after = source.strip_prefix('@')?;
    let end = after.find('@')?;
    let body = &after[..end];
    if body.is_empty() || body.contains('\n') {
        return None;
    }
    let (name, default) = match body.find('=') {
        Some(index) => (1..1 + index, Some(1 + index + 1..1 + end)),
        None => (1..1 + end, None),
    };
    if name.is_empty() {
        return None;
    }
    Some(VariableShape {
        length: end + 2,
        name,
        default,
    })
}
