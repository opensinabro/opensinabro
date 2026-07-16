use crate::grammar::{
    Region, brace, emit_joined_range_as, emit_line_newline, emit_line_prefix, emit_lines_flat,
    inline, table,
};
use crate::kind::SyntaxKind;
use crate::parser::Parser;
use namumark_text as text;
use std::ops::Range;

pub(crate) fn parse_region_blocks(parser: &mut Parser<'_>, region: &Region, list_context: bool) {
    let mut index = 0;
    while index < region.line_count() {
        if !parser.tick() {
            emit_lines_flat(parser, region, index..region.line_count());
            return;
        }
        let line_text = region.line_text(index);

        if line_text.trim().is_empty() {
            emit_line_prefix(parser, region, index);
            parser.emit_token(SyntaxKind::Text, region.lines[index].content.end);
            emit_line_newline(parser, region, index);
            index += 1;
            continue;
        }
        if line_text.starts_with("##") {
            emit_single_line_node(parser, region, index, SyntaxKind::Comment, SyntaxKind::Text);
            index += 1;
            continue;
        }
        if text::parse_redirect(line_text).is_some() {
            emit_single_line_node(
                parser,
                region,
                index,
                SyntaxKind::Redirect,
                SyntaxKind::Text,
            );
            index += 1;
            continue;
        }
        if let Some(shape) = text::heading_shape(line_text) {
            let content = region.lines[index].content.clone();
            let marker = parser.start_node();
            emit_line_prefix(parser, region, index);
            parser.emit_token(SyntaxKind::Marker, content.start + shape.content_start);
            inline::parse_inline_range(
                parser,
                content.start + shape.content_start..content.start + shape.content_end,
            );
            parser.emit_token(SyntaxKind::Marker, content.end);
            marker.complete(parser, SyntaxKind::Heading);
            emit_line_newline(parser, region, index);
            index += 1;
            continue;
        }
        if text::is_horizontal_rule(line_text) {
            emit_single_line_node(
                parser,
                region,
                index,
                SyntaxKind::HorizontalRule,
                SyntaxKind::Marker,
            );
            index += 1;
            continue;
        }
        if line_text.starts_with('>') {
            let start = index;
            let mut consumed = Vec::new();
            while index < region.line_count() {
                let text = region.line_text(index);
                let Some(after) = text.strip_prefix('>') else {
                    break;
                };
                consumed.push(if after.starts_with(' ') { 2 } else { 1 });
                index += 1;
            }
            let sub = region.sub_region(
                parser.source(),
                start..index,
                &consumed,
                SyntaxKind::QuoteMarker,
            );
            let marker = parser.start_node();
            parse_region_blocks(parser, &sub, false);
            marker.complete(parser, SyntaxKind::Quote);
            continue;
        }
        if list_context && text::list_marker(line_text).is_some() {
            index += parse_list(parser, region, index);
            continue;
        }
        if line_text.starts_with(' ') {
            let start = index;
            while index < region.line_count() {
                let text = region.line_text(index);
                if !text.starts_with(' ') || text.trim().is_empty() {
                    break;
                }
                index += 1;
            }
            let consumed = vec![1; index - start];
            let sub = region.sub_region(
                parser.source(),
                start..index,
                &consumed,
                SyntaxKind::IndentMarker,
            );
            if text::list_marker(sub.line_text(0)).is_some() {
                parse_list_and_indent_chunks(parser, &sub);
            } else {
                let marker = parser.start_node();
                parse_region_blocks(parser, &sub, true);
                marker.complete(parser, SyntaxKind::Indent);
            }
            continue;
        }
        if text::is_table_start(line_text)
            && let Some(consumed) = table::try_parse_table(parser, region, index)
        {
            emit_line_newline(parser, region, index + consumed - 1);
            index += consumed;
            continue;
        }

        index = parse_paragraph_like(parser, region, index, list_context);
    }
}

fn emit_single_line_node(
    parser: &mut Parser<'_>,
    region: &Region,
    index: usize,
    node_kind: SyntaxKind,
    content_kind: SyntaxKind,
) {
    let marker = parser.start_node();
    emit_line_prefix(parser, region, index);
    parser.emit_token(content_kind, region.lines[index].content.end);
    marker.complete(parser, node_kind);
    emit_line_newline(parser, region, index);
}

fn is_block_boundary(line: &str, list_context: bool) -> bool {
    line.trim().is_empty()
        || line.starts_with("##")
        || line.starts_with('>')
        || line.starts_with(' ')
        || line.starts_with("{{{")
        || text::parse_redirect(line).is_some()
        || text::heading_shape(line).is_some()
        || text::is_horizontal_rule(line)
        || text::is_table_start(line)
        || (list_context && text::list_marker(line).is_some())
}

// 들여쓰기 영역 내에서 리스트와 일반 블록 청크를 교대로 처리한다.
// 리스트 자체가 들여쓰기를 내포하므로 Indent로 감싸지 않고, 나머지 연속 라인은 Indent로 묶는다.
fn parse_list_and_indent_chunks(parser: &mut Parser<'_>, region: &Region) {
    let mut index = 0;
    while index < region.line_count() {
        if !parser.tick() {
            emit_lines_flat(parser, region, index..region.line_count());
            return;
        }
        if text::list_marker(region.line_text(index)).is_some() {
            index += parse_list(parser, region, index);
        } else {
            let start = index;
            while index < region.line_count()
                && text::list_marker(region.line_text(index)).is_none()
            {
                index += 1;
            }
            let chunk = region.slice_lines(parser.source(), start..index);
            let marker = parser.start_node();
            parse_region_blocks(parser, &chunk, true);
            marker.complete(parser, SyntaxKind::Indent);
        }
    }
}

fn parse_list(parser: &mut Parser<'_>, region: &Region, start: usize) -> usize {
    let Some(first_marker) = text::list_marker(region.line_text(start)) else {
        return 1;
    };
    let kind = first_marker.kind;
    let list_marker = parser.start_node();
    let mut index = start;
    while index < region.line_count() {
        if !parser.tick() {
            break;
        }
        let line_text = region.line_text(index);
        let Some(item) = text::list_marker(line_text) else {
            break;
        };
        if item.kind != kind {
            break;
        }
        let content = item.content;
        let item_marker = parser.start_node();
        emit_line_prefix(parser, region, index);
        let content_range = region.lines[index].content.clone();
        let marker_end = content_range.start + (line_text.len() - content.len());
        parser.emit_token(SyntaxKind::ListMarker, marker_end);
        if !content.is_empty() {
            let paragraph = parser.start_node();
            inline::parse_inline_range(parser, marker_end..content_range.end);
            paragraph.complete(parser, SyntaxKind::Paragraph);
        }
        emit_line_newline(parser, region, index);
        index += 1;

        let continuation_start = index;
        while index < region.line_count() {
            let text = region.line_text(index);
            if !text.starts_with(' ') || text.trim().is_empty() {
                break;
            }
            index += 1;
        }
        if continuation_start < index {
            let consumed = vec![1; index - continuation_start];
            let sub = region.sub_region(
                parser.source(),
                continuation_start..index,
                &consumed,
                SyntaxKind::IndentMarker,
            );
            parse_region_blocks(parser, &sub, true);
        }
        item_marker.complete(parser, SyntaxKind::ListItem);
    }
    list_marker.complete(parser, SyntaxKind::List);
    index - start
}

// ---- 문단 (brace 그룹 세그먼트 포함) ----

fn parse_paragraph_like(
    parser: &mut Parser<'_>,
    region: &Region,
    start: usize,
    list_context: bool,
) -> usize {
    // 문단 중간에서 열린 `{{{` 그룹은 닫힐 때까지 경계를 무시하고 이어 붙인다.
    let mut end = start + 1;
    let mut depth = text::brace_delta(region.line_text(start)).max(0);
    while end < region.line_count() {
        let next = region.line_text(end);
        if depth == 0 && is_block_boundary(next, list_context) {
            break;
        }
        depth = (depth + text::brace_delta(next)).max(0);
        end += 1;
    }
    if depth > 0 {
        // 닫히지 않은 그룹: 경계 규칙만으로 다시 수집한다.
        end = start + 1;
        while end < region.line_count() && !is_block_boundary(region.line_text(end), list_context) {
            end += 1;
        }
    }
    emit_paragraph_segments(parser, region, region.joined_range_of_lines(start..end));
    emit_line_newline(parser, region, end - 1);
    end
}

// 여러 줄에 걸친 `{{{ ... }}}` 그룹을 블록으로 분리하고 나머지는 문단으로 방출한다.
// 한 줄 안에서 닫힌 그룹은 인라인 처리를 위해 문단에 남긴다.
fn emit_paragraph_segments(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    let joined = &region.joined;
    let mut position = joined_range.start;
    let mut segment_start = joined_range.start;
    while position < joined_range.end {
        if !parser.tick() {
            break;
        }
        if joined[position..joined_range.end].starts_with("{{{") {
            let group_source = &joined[position..joined_range.end];
            if let Some(close) = text::find_matching_braces(group_source) {
                let group_end = position + close + 3;
                if group_source[..close + 3].contains('\n') {
                    emit_text_segment(parser, region, segment_start..position);
                    brace::parse_brace_group(parser, region, position..group_end);
                    position = group_end;
                    segment_start = group_end;
                    continue;
                }
                position = group_end;
                continue;
            }
            position += 3;
            continue;
        }
        position += next_char_length(joined, position);
    }
    emit_text_segment(parser, region, segment_start..joined_range.end);
}

// 텍스트 세그먼트: 그룹과 인접한 구조적 개행 하나씩은 문단 밖(부모)으로 방출한다.
fn emit_text_segment(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    if joined_range.is_empty() {
        return;
    }
    let joined = &region.joined;
    let segment = &joined[joined_range.clone()];
    let core_start = if segment.starts_with('\n') {
        joined_range.start + 1
    } else {
        joined_range.start
    };
    let core_end =
        if core_start < joined_range.end && joined[core_start..joined_range.end].ends_with('\n') {
            joined_range.end - 1
        } else {
            joined_range.end
        };

    emit_plain_joined(parser, region, joined_range.start..core_start);
    if joined[core_start..core_end].trim().is_empty() {
        emit_plain_joined(parser, region, core_start..core_end);
    } else {
        let marker = parser.start_node();
        emit_flowing_inline(parser, region, core_start..core_end);
        marker.complete(parser, SyntaxKind::Paragraph);
    }
    emit_plain_joined(parser, region, core_end..joined_range.end);
}

/// 개행·prefix를 존중하며 구조 없는 텍스트로 방출한다.
fn emit_plain_joined(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    emit_joined_range_as(parser, region, joined_range, SyntaxKind::Text);
}

/// 라인 조각마다 인라인 파싱을 수행하고 전환부는 개행+prefix로 방출한다.
fn emit_flowing_inline(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    if joined_range.is_empty() {
        return;
    }
    let sub = region.sub_region_from_joined(parser.source(), joined_range);
    for index in 0..sub.line_count() {
        emit_line_prefix(parser, &sub, index);
        inline::parse_inline_range(parser, sub.lines[index].content.clone());
        emit_line_newline(parser, &sub, index);
    }
}

fn next_char_length(text: &str, position: usize) -> usize {
    text[position..]
        .chars()
        .next()
        .map(char::len_utf8)
        .unwrap_or(1)
}
