use crate::grammar::{
    Region, block, emit_joined_range_as, emit_line_newline, emit_line_prefix, inline,
};
use crate::kind::SyntaxKind;
use crate::parser::Parser;
use namumark_text as text;
use std::ops::Range;

struct RowSource {
    /// 이 행 바로 앞에 있던 주석 줄들. 주석은 표를 끊지 않으므로 행 사이에 끼어 있다.
    leading_comments: Range<usize>,
    line_range: Range<usize>,
    /// 결정용 행 텍스트. 캡션 행은 `|캡션|`이 `||`로 치환된 합성 문자열이다.
    text: String,
    /// 합성 문자열 오프셋 → joined 오프셋 보정값 (캡션 행만 0이 아니다)
    joined_bias: isize,
}

/// 표 시도. 결정을 모두 마친 뒤에만 방출하므로 실패 시 아무것도 방출하지 않는다.
pub(crate) fn try_parse_table(
    parser: &mut Parser<'_>,
    region: &Region,
    start_index: usize,
) -> Option<usize> {
    let line_count = region.line_count();
    let first_text = region.line_text(start_index);
    let caption = text::caption_range(first_text);
    if caption.is_none() && !first_text.starts_with("||") {
        return None;
    }

    let mut rows: Vec<RowSource> = Vec::new();
    let mut index = start_index;
    while index < line_count {
        if !parser.tick() {
            break;
        }
        // 주석 줄은 표를 끊지 않는다 — 나무위키는 주석을 지우고 나머지를 이어 읽는다
        // (렌더확정: 행 사이의 `## 내용` 줄을 사이에 두고도 the seed의 표가 이어진다).
        let comments_start = index;
        while index < line_count
            && region.lines[index].prefix.is_empty()
            && region.line_text(index).starts_with("##")
            && !rows.is_empty()
        {
            index += 1;
        }
        let leading_comments = comments_start..index;
        if index >= line_count {
            break;
        }
        let row_start = index;
        let (mut row_text, joined_bias) = if index == start_index {
            match &caption {
                Some(caption) => {
                    let after = &first_text[caption.end + 1..];
                    (format!("||{after}"), (caption.end + 1) as isize - 2)
                }
                None => (first_text.to_string(), 0),
            }
        } else {
            let text = region.line_text(index);
            if !text.starts_with("||") {
                break;
            }
            (text.to_string(), 0)
        };
        index += 1;
        while !text::is_row_complete(&row_text) && index < line_count {
            row_text.push('\n');
            row_text.push_str(region.line_text(index));
            index += 1;
        }
        if !text::is_row_complete(&row_text) && rows.is_empty() {
            return None;
        }
        rows.push(RowSource {
            leading_comments: leading_comments.clone(),
            line_range: row_start..index,
            text: row_text,
            joined_bias,
        });
    }

    let split_rows: Vec<Vec<(usize, Range<usize>)>> = rows
        .iter()
        .map(|row| text::split_cell_ranges(row.text.trim_end()))
        .collect();
    if split_rows.iter().all(|cells| cells.is_empty()) {
        return None;
    }

    let table_marker = parser.start_node();
    let row_count = rows.len();
    for (row_index, (row, cells)) in rows.iter().zip(&split_rows).enumerate() {
        emit_row(
            parser,
            region,
            row,
            cells,
            if row_index == 0 {
                caption.as_ref()
            } else {
                None
            },
        );
        if row_index + 1 < row_count {
            emit_line_newline(parser, region, row.line_range.end - 1);
        }
    }
    table_marker.complete(parser, SyntaxKind::Table);
    Some(index - start_index)
}

fn emit_row(
    parser: &mut Parser<'_>,
    region: &Region,
    row: &RowSource,
    cells: &[(usize, Range<usize>)],
    caption: Option<&Range<usize>>,
) {
    for line in row.leading_comments.clone() {
        let marker = parser.start_node();
        emit_line_prefix(parser, region, line);
        parser.emit_token(SyntaxKind::Text, region.lines[line].content.end);
        marker.complete(parser, SyntaxKind::Comment);
        emit_line_newline(parser, region, line);
    }
    let first_line = row.line_range.start;
    let row_joined_start = region.joined_start(first_line);
    // 합성 행 텍스트 오프셋 → joined 오프셋. 캡션 행의 가상 `||`(offset < 2)에는 쓰지 않는다.
    let to_joined = |offset: usize| -> usize {
        (row_joined_start as isize + offset as isize + row.joined_bias) as usize
    };

    let row_marker = parser.start_node();
    emit_line_prefix(parser, region, first_line);

    if let Some(caption) = caption {
        let line_start = region.lines[first_line].content.start;
        let caption_node = parser.start_node();
        parser.emit_token(SyntaxKind::Marker, line_start + caption.start);
        inline::parse_inline_range(parser, line_start + caption.start..line_start + caption.end);
        parser.emit_token(SyntaxKind::Marker, line_start + caption.end + 1);
        caption_node.complete(parser, SyntaxKind::TableCaption);
    }

    let trimmed_length = row.text.trim_end().len();
    // 선행 파이프 런 (캡션 행은 합성 `||` 2바이트를 제외한 실제 부분만)
    let leading_run_end = cells
        .first()
        .map(|(_, range)| range.start)
        .unwrap_or(trimmed_length);
    let real_run_start = if caption.is_some() { 2 } else { 0 };
    if leading_run_end > real_run_start {
        emit_joined_range_as(
            parser,
            region,
            to_joined(real_run_start)..to_joined(leading_run_end),
            SyntaxKind::Marker,
        );
    }

    for (cell_index, (_, cell_range)) in cells.iter().enumerate() {
        emit_cell(parser, region, row, &to_joined, cell_range);
        // 셀 뒤 파이프 런 (다음 셀의 시작 또는 행 끝까지)
        let run_start = cell_range.end;
        let run_end = cells
            .get(cell_index + 1)
            .map(|(_, next)| next.start)
            .unwrap_or(trimmed_length);
        if run_end > run_start {
            emit_joined_range_as(
                parser,
                region,
                to_joined(run_start)..to_joined(run_end),
                SyntaxKind::Marker,
            );
        }
    }
    row_marker.complete(parser, SyntaxKind::TableRow);
}

fn emit_cell(
    parser: &mut Parser<'_>,
    region: &Region,
    row: &RowSource,
    to_joined: &dyn Fn(usize) -> usize,
    cell_range: &Range<usize>,
) {
    let cell_text = &row.text[cell_range.clone()];
    let semantics = text::cell_shape(cell_text);

    let cell_marker = parser.start_node();
    if semantics.options_end > 0 {
        emit_joined_range_as(
            parser,
            region,
            to_joined(cell_range.start)..to_joined(cell_range.start + semantics.options_end),
            SyntaxKind::Marker,
        );
    }
    if semantics.content_start > semantics.options_end {
        emit_joined_range_as(
            parser,
            region,
            to_joined(cell_range.start + semantics.options_end)
                ..to_joined(cell_range.start + semantics.content_start),
            SyntaxKind::Marker,
        );
    }
    if semantics.content_end > semantics.content_start {
        let content_joined = to_joined(cell_range.start + semantics.content_start)
            ..to_joined(cell_range.start + semantics.content_end);
        let sub = region.sub_region_from_joined(parser.source(), content_joined);
        let sub = sub.reclaim_prefixes(parser.source());
        block::parse_region_blocks(parser, &sub, block::RegionContext::Fresh);
    }
    if cell_range.start + semantics.content_end < cell_range.end {
        emit_joined_range_as(
            parser,
            region,
            to_joined(cell_range.start + semantics.content_end)..to_joined(cell_range.end),
            SyntaxKind::Marker,
        );
    }
    cell_marker.complete(parser, SyntaxKind::TableCell);
}
