use crate::grammar::{Region, block, emit_joined_range_as, emit_line_newline, emit_line_prefix};
use crate::kind::SyntaxKind;
use crate::parser::Parser;
use namumark_text as text;
use std::ops::Range;

/// 여러 줄에 걸쳐 정확히 균형 잡힌 `{{{ ... }}}` 그룹(joined 범위)을 블록 노드로 방출한다.
/// `closed`가 거짓이면 `}}}`가 없는 그룹이다 — 어디서도 안 닫히는 `{{{`는 남은 범위를
/// 통째로 머금는다(렌더확정: 원문이 어긋난 표 셀에서 the seed의 `#!folding`이 그렇게 한다).
pub(crate) fn parse_brace_group(
    parser: &mut Parser<'_>,
    region: &Region,
    joined_range: Range<usize>,
    closed: bool,
) {
    let group_joined = joined_range.start;
    let group_global = region.to_global(group_joined);
    let group = region.joined[joined_range.clone()].to_string();
    let header_end_relative = group.find('\n').unwrap_or(group.len());
    let header = group[3..header_end_relative].to_string();
    let closing_joined = if closed {
        joined_range.end - 3
    } else {
        joined_range.end
    };
    let (header_line, header_column) = region.locate(group_joined);

    // 헤더 내 상대 오프셋 → 전역/joined 오프셋
    let header_global = |offset: usize| group_global + 3 + offset;
    let header_joined = |offset: usize| group_joined + 3 + offset;

    let node = parser.start_node();
    if header_column == 0 {
        emit_line_prefix(parser, region, header_line);
    }

    let kind;
    if text::strip_directive(&header, "#!syntax").is_some() {
        kind = SyntaxKind::CodeBlock;
        // 헤더 전체(지시자+언어)가 마커, 내용은 원문 그대로
        parser.emit_token(SyntaxKind::Marker, header_global(header.len()));
        emit_line_newline(parser, region, header_line);
        let content_start = region.joined_start(header_line + 1);
        emit_joined_range_as(
            parser,
            region,
            content_start..closing_joined,
            SyntaxKind::Text,
        );
    } else if let Some(rest) = text::strip_directive(&header, "#!wiki") {
        kind = SyntaxKind::WikiStyle;
        let (_, _, leftover) = text::parse_wiki_style_attributes(rest);
        let leftover_offset = if leftover.is_empty() {
            header.len()
        } else {
            subslice_offset(&header, leftover)
        };
        parse_marked_container(
            parser,
            region,
            header_line,
            leftover.len(),
            header_global(leftover_offset),
            header_joined(leftover_offset),
            closing_joined,
            header_global(header.len()),
        );
    } else if let Some(rest) = text::strip_directive(&header, "#!if") {
        // 헤더 나머지가 조건식이다. 해석은 렌더 단계(resolve)가 한다.
        kind = SyntaxKind::Conditional;
        let expression_offset = if rest.is_empty() {
            header.len()
        } else {
            subslice_offset(&header, rest)
        };
        parser.emit_token(SyntaxKind::Marker, header_global(expression_offset));
        if !rest.is_empty() {
            let expression = parser.start_node();
            parser.emit_token(SyntaxKind::Text, header_global(header.len()));
            expression.complete(parser, SyntaxKind::ConditionExpression);
        }
        emit_line_newline(parser, region, header_line);
        let content_start = region.joined_start(header_line + 1);
        parse_content_blocks(parser, region, content_start..closing_joined, None);
    } else if let Some(rest) = text::strip_directive(&header, "#!folding") {
        kind = SyntaxKind::Folding;
        let summary = rest.trim();
        let summary_offset = if summary.is_empty() {
            header.len()
        } else {
            subslice_offset(&header, summary)
        };
        parser.emit_token(SyntaxKind::Marker, header_global(summary_offset));
        // 접기 문구에는 위키 문법이 적용되지 않는다 — 글자 그대로다(렌더확정: the seed는
        // 문구에 쓴 서식 마커를 풀지 않고 그대로 보여 준다).
        if !summary.is_empty() {
            let summary_node = parser.start_node();
            parser.emit_token(
                SyntaxKind::Text,
                header_global(summary_offset + summary.len()),
            );
            summary_node.complete(parser, SyntaxKind::FoldingSummary);
        }
        emit_line_newline(parser, region, header_line);
        let content_start = region.joined_start(header_line + 1);
        parse_content_blocks(parser, region, content_start..closing_joined, None);
    } else if let Some(rest) = text::strip_directive(&header, "#!html") {
        kind = SyntaxKind::HtmlBlock;
        if rest.is_empty() {
            parser.emit_token(SyntaxKind::Marker, header_global(header.len()));
            emit_line_newline(parser, region, header_line);
            let content_start = region.joined_start(header_line + 1);
            emit_joined_range_as(
                parser,
                region,
                content_start..closing_joined,
                SyntaxKind::Text,
            );
        } else {
            let leftover_offset = subslice_offset(&header, rest);
            parser.emit_token(SyntaxKind::Marker, header_global(leftover_offset));
            emit_joined_range_as(
                parser,
                region,
                header_joined(leftover_offset)..closing_joined,
                SyntaxKind::Text,
            );
        }
    } else if let Some((_, rest)) = text::parse_size_marker(&header) {
        kind = SyntaxKind::SizedBlock;
        let leftover_offset = header.len() - rest.len();
        parse_marked_container(
            parser,
            region,
            header_line,
            rest.len(),
            header_global(leftover_offset),
            header_joined(leftover_offset),
            closing_joined,
            header_global(header.len()),
        );
    } else if text::parse_color_specification(&header).is_some() {
        kind = SyntaxKind::ColoredBlock;
        // 헤더에 공백이 있으면 그 뒤는 내용의 첫 조각이다(`{{{#red 빨강\n…`).
        let rest = header.split_once(' ').map_or("", |(_, rest)| rest);
        let leftover_offset = header.len() - rest.len();
        parse_marked_container(
            parser,
            region,
            header_line,
            rest.len(),
            header_global(leftover_offset),
            header_joined(leftover_offset),
            closing_joined,
            header_global(header.len()),
        );
    } else {
        // 지시자 없는 여러 줄 리터럴. 헤더 텍스트가 있으면 첫 내용 줄이 된다.
        kind = SyntaxKind::CodeBlock;
        parser.emit_token(SyntaxKind::Marker, group_global + 3);
        emit_joined_range_as(
            parser,
            region,
            group_joined + 3..closing_joined,
            SyntaxKind::Text,
        );
    }

    // 닫는 `}}}`
    if closed {
        let closing_global = region.to_global(closing_joined);
        parser.emit_token(SyntaxKind::Marker, closing_global + 3);
    }
    node.complete(parser, kind);
}

/// 헤더가 마커이고 헤더 잔여 텍스트가 첫 내용 줄이 되는 컨테이너(#!wiki, 색상, 크기)의 공통 처리.
#[allow(clippy::too_many_arguments)]
fn parse_marked_container(
    parser: &mut Parser<'_>,
    region: &Region,
    header_line: usize,
    leftover_length: usize,
    leftover_global: usize,
    leftover_joined: usize,
    closing_joined: usize,
    header_end_global: usize,
) {
    if leftover_length == 0 {
        parser.emit_token(SyntaxKind::Marker, header_end_global);
        emit_line_newline(parser, region, header_line);
        let content_start = region.joined_start(header_line + 1);
        parse_content_blocks(parser, region, content_start..closing_joined, None);
    } else {
        parser.emit_token(SyntaxKind::Marker, leftover_global);
        parse_content_blocks(
            parser,
            region,
            leftover_joined..closing_joined,
            Some(leftover_length),
        );
    }
}

/// 내용 joined 범위를 하위 영역으로 만들어 블록을 파싱한다.
/// `first_piece_length`가 주어지면 첫 조각(헤더 잔여)을 그 길이로 잘라
/// 잔여 뒤 공백이 결정 문자열에 들어가지 않게 한다(옛 파서의 trim과 동일).
fn parse_content_blocks(
    parser: &mut Parser<'_>,
    region: &Region,
    content_joined: Range<usize>,
    first_piece_length: Option<usize>,
) {
    if content_joined.is_empty() {
        return;
    }
    let mut sub = region.sub_region_from_joined(parser.source(), content_joined);
    if let Some(length) = first_piece_length {
        let mut lines = std::mem::take(&mut sub.lines);
        if let Some(first) = lines.first_mut() {
            first.content.end = (first.content.start + length).min(first.content.end);
        }
        sub = Region::new(parser.source(), lines);
    }
    let sub = sub.reclaim_prefixes(parser.source());
    block::parse_region_blocks(parser, &sub, block::RegionContext::Fresh);
}

fn subslice_offset(outer: &str, inner: &str) -> usize {
    inner.as_ptr() as usize - outer.as_ptr() as usize
}
