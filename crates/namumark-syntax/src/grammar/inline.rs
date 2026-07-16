use crate::kind::SyntaxKind;
use crate::parser::Parser;
use namumark_text as text;
use std::ops::Range;

const STYLE_MARKERS: &[(&str, SyntaxKind)] = &[
    ("'''", SyntaxKind::Bold),
    ("''", SyntaxKind::Italic),
    ("~~", SyntaxKind::Strikethrough),
    ("--", SyntaxKind::Strikethrough),
    ("__", SyntaxKind::Underline),
    ("^^", SyntaxKind::Superscript),
    (",,", SyntaxKind::Subscript),
];

/// 단일 라인 범위(개행 없음)의 인라인 구문을 파싱한다.
/// 구문으로 소비되지 않은 문자는 누적되어 Text 토큰으로 방출된다.
/// (parser.position이 마지막 방출 지점이므로, 구문 시작 전 `emit_token(Text, ...)`이 곧 flush다.)
pub(crate) fn parse_inline_range(parser: &mut Parser<'_>, range: Range<usize>) {
    let mut position = range.start;
    while position < range.end {
        if !parser.tick() {
            break;
        }
        let source = parser.source();
        let rest = &source[position..range.end];
        let consumed = if rest.starts_with('\\') {
            consume_escape(parser, position, range.end)
        } else if rest.starts_with("{{{") {
            consume_literal(parser, position, range.end)
        } else if rest.starts_with("[[") {
            consume_link(parser, position, range.end)
        } else if rest.starts_with("[*") {
            consume_footnote(parser, position, range.end)
        } else if rest.starts_with('[') {
            consume_macro(parser, position, range.end)
        } else {
            consume_styled(parser, position, range.end)
        };
        position += if consumed > 0 {
            consumed
        } else {
            next_char_length(parser.source(), position)
        };
    }
    parser.emit_token(SyntaxKind::Text, range.end);
}

fn consume_escape(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    let mut characters = parser.source()[position..end].chars();
    characters.next();
    let Some(escaped) = characters.next() else {
        return 0;
    };
    let length = 1 + escaped.len_utf8();
    parser.emit_token(SyntaxKind::Text, position);
    parser.emit_token(SyntaxKind::Escaped, position + length);
    length
}

fn consume_literal(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    let rest = &parser.source()[position..end];
    let Some(close) = text::find_matching_braces(rest) else {
        return 0;
    };
    let content_range = position + 3..position + close;
    let consumed = close + 3;
    let content = &parser.source()[content_range.clone()];

    parser.emit_token(SyntaxKind::Text, position);
    let marker = parser.start_node();
    if let Some(html) = content.strip_prefix("#!html ") {
        let html_start = content_range.end - html.len();
        parser.emit_token(SyntaxKind::Marker, html_start);
        parser.emit_token(SyntaxKind::Text, content_range.end);
        parser.emit_token(SyntaxKind::Marker, position + consumed);
        marker.complete(parser, SyntaxKind::InlineHtml);
    } else if let Some((_, inner)) = text::parse_size_marker(content) {
        let inner_start = content_range.end - inner.len();
        parser.emit_token(SyntaxKind::Marker, inner_start);
        parse_inline_range(parser, inner_start..content_range.end);
        parser.emit_token(SyntaxKind::Marker, position + consumed);
        marker.complete(parser, SyntaxKind::SizedText);
    } else if let Some((_, _, inner)) = text::parse_color_marker(content) {
        let inner_start = content_range.end - inner.len();
        parser.emit_token(SyntaxKind::Marker, inner_start);
        parse_inline_range(parser, inner_start..content_range.end);
        parser.emit_token(SyntaxKind::Marker, position + consumed);
        marker.complete(parser, SyntaxKind::ColoredText);
    } else {
        parser.emit_token(SyntaxKind::Marker, content_range.start);
        parser.emit_token(SyntaxKind::Text, content_range.end);
        parser.emit_token(SyntaxKind::Marker, position + consumed);
        marker.complete(parser, SyntaxKind::Literal);
    }
    consumed
}

fn consume_link(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    let rest = &parser.source()[position..end];
    let Some(close) = text::find_matching_double_bracket(rest) else {
        return 0;
    };
    let body = &rest[2..close];
    if body.is_empty() {
        return 0;
    }
    let consumed = close + 2;
    let (target, display) = match body.split_once('|') {
        Some((target, display)) => (target, Some(display)),
        None => (body, None),
    };
    let kind = if text::strip_link_prefix(target, &["파일:", "file:"]).is_some() {
        SyntaxKind::Image
    } else if text::strip_link_prefix(target, &["분류:", "category:"]).is_some() {
        SyntaxKind::Category
    } else {
        SyntaxKind::Link
    };

    parser.emit_token(SyntaxKind::Text, position);
    let marker = parser.start_node();
    match (kind, display) {
        (SyntaxKind::Link, Some(display)) => {
            // `[[대상|` 까지 마커, 표시부는 인라인 자식으로
            let display_start = position + 2 + target.len() + 1;
            parser.emit_token(SyntaxKind::Marker, display_start);
            parse_inline_range(parser, display_start..display_start + display.len());
            parser.emit_token(SyntaxKind::Marker, position + consumed);
        }
        _ => {
            parser.emit_token(SyntaxKind::Marker, position + consumed);
        }
    }
    marker.complete(parser, kind);
    consumed
}

fn consume_footnote(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    let rest = &parser.source()[position..end];
    let Some(close) = text::find_matching_bracket(rest) else {
        return 0;
    };
    let consumed = close + 1;
    let body = &rest[2..close];

    parser.emit_token(SyntaxKind::Text, position);
    let marker = parser.start_node();
    match body.split_once(' ') {
        Some((name, content)) => {
            // `[*이름 ` 까지 마커
            let content_start = position + 2 + name.len() + 1;
            parser.emit_token(SyntaxKind::Marker, content_start);
            parse_inline_range(parser, content_start..content_start + content.len());
            parser.emit_token(SyntaxKind::Marker, position + consumed);
        }
        None => {
            parser.emit_token(SyntaxKind::Marker, position + consumed);
        }
    }
    marker.complete(parser, SyntaxKind::Footnote);
    consumed
}

fn consume_macro(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    let rest = &parser.source()[position..end];
    let Some(close) = text::find_matching_bracket(rest) else {
        return 0;
    };
    let body = &rest[1..close];
    let name = match body.split_once('(') {
        Some((name, argument)) => {
            if !argument.ends_with(')') {
                return 0;
            }
            name
        }
        None => body,
    };
    if name.is_empty() || !name.chars().all(char::is_alphanumeric) {
        return 0;
    }
    let consumed = close + 1;
    parser.emit_token(SyntaxKind::Text, position);
    let marker = parser.start_node();
    parser.emit_token(SyntaxKind::Marker, position + consumed);
    marker.complete(parser, SyntaxKind::MacroCall);
    consumed
}

fn consume_styled(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    for &(marker_text, kind) in STYLE_MARKERS {
        let rest = &parser.source()[position..end];
        if !rest.starts_with(marker_text) {
            continue;
        }
        let inner = &rest[marker_text.len()..];
        let Some(offset) = inner.find(marker_text) else {
            continue;
        };
        if offset == 0 {
            continue;
        }
        let content_start = position + marker_text.len();
        let consumed = marker_text.len() * 2 + offset;

        parser.emit_token(SyntaxKind::Text, position);
        let marker = parser.start_node();
        parser.emit_token(SyntaxKind::Marker, content_start);
        parse_inline_range(parser, content_start..content_start + offset);
        parser.emit_token(SyntaxKind::Marker, position + consumed);
        marker.complete(parser, kind);
        return consumed;
    }
    0
}

fn next_char_length(text: &str, position: usize) -> usize {
    text[position..]
        .chars()
        .next()
        .map(char::len_utf8)
        .unwrap_or(1)
}
