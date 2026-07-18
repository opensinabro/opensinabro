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
        let consumed = if rest.starts_with('\n') {
            // 인라인 범위가 여러 줄에 걸칠 수 있다 — 각주나 `{{{` 그룹이 줄을 넘는다.
            parser.emit_token(SyntaxKind::Text, position);
            parser.emit_token(SyntaxKind::Newline, position + 1);
            1
        } else if rest.starts_with('\\') {
            consume_escape(parser, position, range.end)
        } else if rest.starts_with("{{{") {
            consume_literal(parser, position, range.end)
        } else if rest.starts_with("[[") {
            consume_link(parser, position, range.end)
        } else if rest.starts_with("[*") {
            consume_footnote(parser, position, range.end)
        } else if rest.starts_with('@') {
            consume_variable(parser, position, range.end)
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
        parser.emit_token(SyntaxKind::DelimiterOpen, content_range.start);
        parser.emit_token(SyntaxKind::Directive, html_start);
        parser.emit_token(SyntaxKind::Text, content_range.end);
        parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        marker.complete(parser, SyntaxKind::InlineHtml);
    } else if let Some(rest) = text::strip_directive(content, "#!wiki") {
        // 링크 표시부처럼 인라인 문맥에서 열린 `#!wiki`. 헤더 잔여와 뒷줄이 모두 내용이라
        // 문단 하나로 묶는다 — lower가 블록을 찾기 때문이다.
        let (_, _, leftover) = text::parse_wiki_style_attributes(rest);
        // 잔여는 양쪽이 다듬긴 조각이라 길이로 자리를 되짚으면 글자 가운데를 밟는다.
        let content_start =
            content_range.start + (leftover.as_ptr() as usize - content.as_ptr() as usize);
        let attributes_start =
            content_range.start + (rest.as_ptr() as usize - content.as_ptr() as usize);
        parser.emit_token(SyntaxKind::DelimiterOpen, content_range.start);
        parser.emit_token(SyntaxKind::Directive, content_range.start + "#!wiki".len());
        if attributes_start > content_range.start + "#!wiki".len() {
            parser.emit_token(SyntaxKind::Separator, attributes_start);
        }
        if content_start > attributes_start {
            parser.emit_token(SyntaxKind::WikiAttributes, content_start);
        }
        let paragraph = parser.start_node();
        parse_inline_range(parser, content_start..content_range.end);
        paragraph.complete(parser, SyntaxKind::Paragraph);
        parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        marker.complete(parser, SyntaxKind::WikiStyle);
    } else if let Some((_, inner)) = text::parse_size_marker(content) {
        let inner_start = content_range.end - inner.len();
        // 크기 단계는 부호+한 자리라 항상 2바이트다.
        parser.emit_token(SyntaxKind::DelimiterOpen, content_range.start);
        parser.emit_token(SyntaxKind::SizeLevel, content_range.start + 2);
        parser.emit_token(SyntaxKind::Separator, inner_start);
        parse_inline_range(parser, inner_start..content_range.end);
        parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        marker.complete(parser, SyntaxKind::SizedText);
    } else if let Some((_, _, inner)) = text::parse_color_marker(content) {
        let inner_start = content_range.end - inner.len();
        parser.emit_token(SyntaxKind::DelimiterOpen, content_range.start);
        // 색상 값 뒤에는 내용을 가르는 공백이 반드시 있다(parse_color_marker 계약).
        parser.emit_token(SyntaxKind::ColorValue, inner_start - 1);
        parser.emit_token(SyntaxKind::Separator, inner_start);
        parse_inline_range(parser, inner_start..content_range.end);
        parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        marker.complete(parser, SyntaxKind::ColoredText);
    } else {
        parser.emit_token(SyntaxKind::DelimiterOpen, content_range.start);
        parser.emit_token(SyntaxKind::Text, content_range.end);
        parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
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
    let (target, display) = text::split_link_body(body);
    let kind = if text::strip_link_prefix(target, &["파일:", "file:"]).is_some() {
        SyntaxKind::Image
    } else if text::strip_link_prefix(target, &["분류:", "category:"]).is_some() {
        SyntaxKind::Category
    } else {
        SyntaxKind::Link
    };

    parser.emit_token(SyntaxKind::Text, position);
    let marker = parser.start_node();
    parser.emit_token(SyntaxKind::DelimiterOpen, position + 2);
    parser.emit_token(SyntaxKind::LinkTarget, position + 2 + target.len());
    match display {
        Some(display) => {
            let display_start = position + 2 + target.len() + 1;
            parser.emit_token(SyntaxKind::Separator, display_start);
            // 이미지의 표시부는 `width=200&align=center` 같은 옵션 목록이라 옵션 단위로 쪼갠다.
            // 링크·분류의 표시부는 인라인 내용이므로 그대로 파싱한다.
            if kind == SyntaxKind::Image {
                emit_argument_list(parser, display_start, display, '&');
            } else {
                parse_inline_range(parser, display_start..display_start + display.len());
            }
            parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        }
        None => {
            parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        }
    }
    marker.complete(parser, kind);
    consumed
}

/// `id,width=480&…` 같은 인자·옵션 목록을 항목 구분자로 쪼갠다.
/// 각 항목은 `이름=값`이면 이름·`=`·값으로, 아니면 하나의 인자로 방출한다.
/// `start`는 목록 첫 글자의 원문 오프셋이다.
fn emit_argument_list(parser: &mut Parser<'_>, start: usize, list: &str, separator: char) {
    let mut cursor = start;
    let mut first = true;
    for item in list.split(separator) {
        if !first {
            parser.emit_token(SyntaxKind::Separator, cursor + separator.len_utf8());
            cursor += separator.len_utf8();
        }
        first = false;
        match item.split_once('=') {
            Some((name, _)) => {
                parser.emit_token(SyntaxKind::ArgumentName, cursor + name.len());
                parser.emit_token(SyntaxKind::Separator, cursor + name.len() + 1);
                parser.emit_token(SyntaxKind::ArgumentValue, cursor + item.len());
            }
            None => {
                parser.emit_token(SyntaxKind::MacroArgument, cursor + item.len());
            }
        }
        cursor += item.len();
    }
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
    parser.emit_token(SyntaxKind::DelimiterOpen, position + 2);
    match body.split_once(' ') {
        Some((name, content)) => {
            parser.emit_token(SyntaxKind::FootnoteName, position + 2 + name.len());
            let content_start = position + 2 + name.len() + 1;
            parser.emit_token(SyntaxKind::Separator, content_start);
            parse_inline_range(parser, content_start..content_start + content.len());
            parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        }
        None => {
            parser.emit_token(SyntaxKind::FootnoteName, position + 2 + body.len());
            parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
        }
    }
    marker.complete(parser, SyntaxKind::Footnote);
    consumed
}

/// `@이름@` / `@이름=기본값@`. 값은 렌더 단계에서 정해진다.
fn consume_variable(parser: &mut Parser<'_>, position: usize, end: usize) -> usize {
    let Some(shape) = text::variable_shape(&parser.source()[position..end]) else {
        return 0;
    };
    parser.emit_token(SyntaxKind::Text, position);
    let node = parser.start_node();
    parser.emit_token(SyntaxKind::DelimiterOpen, position + 1);
    parser.emit_token(SyntaxKind::VariableName, position + shape.name.end);
    match &shape.default {
        Some(default) => {
            parser.emit_token(SyntaxKind::Separator, position + default.start);
            parser.emit_token(SyntaxKind::VariableDefault, position + default.end);
            parser.emit_token(SyntaxKind::DelimiterClose, position + shape.length);
        }
        None => {
            parser.emit_token(SyntaxKind::DelimiterClose, position + shape.length);
        }
    }
    node.complete(parser, SyntaxKind::TemplateVariable);
    shape.length
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
    parser.emit_token(SyntaxKind::DelimiterOpen, position + 1);
    parser.emit_token(SyntaxKind::MacroName, position + 1 + name.len());
    if name.len() < body.len() {
        // 인자 있음: `(인자)`. 여는·닫는 괄호는 구분자, 사이는 쉼표로 갈린 인자 목록이다.
        let paren_open = position + 1 + name.len();
        parser.emit_token(SyntaxKind::Separator, paren_open + 1);
        let arguments = &body[name.len() + 1..body.len() - 1];
        emit_argument_list(parser, paren_open + 1, arguments, ',');
        parser.emit_token(SyntaxKind::Separator, position + 1 + body.len());
    }
    parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
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
        parser.emit_token(SyntaxKind::DelimiterOpen, content_start);
        parse_inline_range(parser, content_start..content_start + offset);
        parser.emit_token(SyntaxKind::DelimiterClose, position + consumed);
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
