use crate::grammar::{
    Region, brace, emit_line_newline, emit_line_prefix, emit_lines_flat, inline, table,
};
use crate::kind::SyntaxKind;
use crate::parser::Parser;
use namumark_text as text;
use std::ops::Range;

/// 이 영역을 어떤 자리에서 읽는가. 들여쓰기 한 칸이 리스트 제 것인지 진짜 한 단계인지가
/// 여기서 갈린다.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegionContext {
    /// 문서·컨테이너·표 셀·인용문처럼 새로 시작하는 자리. 줄머리 공백은 아직 아무도
    /// 먹지 않았으므로 리스트 마커 앞 한 칸은 리스트 제 것이다.
    Fresh,
    /// 들여쓰기로 열린 영역. 줄머리 마커는 곧 리스트다.
    Indented,
    /// 리스트 항목의 속내용. 리스트가 이미 한 칸을 먹었으므로 여기 남은 들여쓰기는
    /// 진짜 한 단계다(렌더확정: ` *가로정렬` 안의 `   *<(>`가 `<div class='wiki-indent'><ul>`).
    ListItem,
}

impl RegionContext {
    /// 줄머리 리스트 마커가 곧 리스트인가.
    fn opens_lists(self) -> bool {
        self != RegionContext::Fresh
    }
}

pub(crate) fn parse_region_blocks(
    parser: &mut Parser<'_>,
    region: &Region,
    context: RegionContext,
) {
    let mut index = 0;
    while index < region.line_count() {
        if !parser.tick() {
            emit_lines_flat(parser, region, index..region.line_count());
            return;
        }
        let line_text = region.line_text(index);

        if line_text.trim().is_empty() {
            // 블록 뒤 빈 줄의 개행은 뒤따르는 문단의 첫 줄바꿈이 된다
            // (the seed: `</ul>` 뒤 빈 줄 → `<div class='wiki-paragraph'><br>…`).
            if paragraph_follows(region, index, context) {
                index = parse_paragraph_like(parser, region, index, context);
            } else {
                // 블록 사이에 남은 빈 줄은 빈 문단이 된다(렌더확정: 표와 다음 헤딩
                // 사이의 빈 줄이 the seed에서 `<div class='wiki-paragraph'></div>`다).
                // 연속된 빈 줄은 문단을 여럿 만들지 않고 하나로 합치며, (줄 수 - 1)개의
                // 개행이 그 안에 br로 남는다(렌더확정: the seed는 빈 문단을 연달아 내지
                // 않는다 — 빈 문단 0건, 빈 줄 2개 자리는 `<div class='wiki-paragraph'><br></div>`).
                let paragraph = parser.start_node();
                emit_line_prefix(parser, region, index);
                parser.emit_token(SyntaxKind::Text, region.lines[index].content.end);
                while index + 1 < region.line_count()
                    && region.line_text(index + 1).trim().is_empty()
                {
                    emit_line_newline(parser, region, index);
                    index += 1;
                    emit_line_prefix(parser, region, index);
                    parser.emit_token(SyntaxKind::Text, region.lines[index].content.end);
                }
                paragraph.complete(parser, SyntaxKind::Paragraph);
                emit_line_newline(parser, region, index);
                index += 1;
            }
            continue;
        }
        if is_comment(region, index) {
            emit_comment_line(parser, region, index);
            index += 1;
            continue;
        }
        if text::parse_redirect(line_text).is_some() {
            emit_redirect_line(parser, region, index);
            index += 1;
            continue;
        }
        if let Some(shape) = text::heading_shape(line_text) {
            let content = region.lines[index].content.clone();
            let marker = parser.start_node();
            emit_line_prefix(parser, region, index);
            // 여는 `==`/`==#` 뒤에 공백 1칸, 내용, 공백 1칸, 닫는 `==`/`#==`.
            parser.emit_token(
                SyntaxKind::DelimiterOpen,
                content.start + shape.content_start - 1,
            );
            parser.emit_token(SyntaxKind::Separator, content.start + shape.content_start);
            inline::parse_inline_range(
                parser,
                content.start + shape.content_start..content.start + shape.content_end,
            );
            parser.emit_token(SyntaxKind::Separator, content.start + shape.content_end + 1);
            parser.emit_token(SyntaxKind::DelimiterClose, content.end);
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
            let consumed = collect_marked_lines(region, &mut index, quote_marker_length);
            let sub = region.sub_region(
                parser.source(),
                start..index,
                &consumed,
                SyntaxKind::QuoteMarker,
            );
            let marker = parser.start_node();
            parse_region_blocks(parser, &sub, RegionContext::Fresh);
            marker.complete(parser, SyntaxKind::Quote);
            continue;
        }
        if context.opens_lists() && text::list_marker(line_text).is_some() {
            index += parse_list(parser, region, index);
            continue;
        }
        if line_text.starts_with(' ') {
            let start = index;
            let mut consumed =
                collect_marked_lines_with(region, &mut index, indent_marker_length, OpenRow::Holds);
            // 리스트 항목 속내용에서는 뒤따르는 빈 줄도 이 들여쓰기 안에 남는다
            // (렌더확정: the seed가 `</ul><div class='wiki-paragraph'></div></div></li>`로 낸다).
            if context == RegionContext::ListItem {
                while index < region.line_count() && region.line_text(index).trim().is_empty() {
                    consumed.push(0);
                    index += 1;
                }
            }
            let sub = region.sub_region(
                parser.source(),
                start..index,
                &consumed,
                SyntaxKind::IndentMarker,
            );
            // 리스트 항목 속내용에서 또 들여쓴 것만 들여쓰기 한 단계다 — 새로 시작하는
            // 자리의 첫 한 칸은 리스트 마커 제 것이라 감싸지 않는다.
            let list_ahead = text::list_marker(sub.line_text(0)).is_some();
            if list_ahead && context != RegionContext::ListItem {
                parse_list_and_indent_chunks(parser, &sub);
                continue;
            }
            let marker = parser.start_node();
            if list_ahead {
                parse_list_and_indent_chunks(parser, &sub);
            } else {
                parse_region_blocks(parser, &sub, RegionContext::Indented);
            }
            marker.complete(parser, SyntaxKind::Indent);
            continue;
        }
        if text::is_table_start(line_text)
            && let Some(consumed) = table::try_parse_table(parser, region, index)
        {
            emit_line_newline(parser, region, index + consumed - 1);
            index += consumed;
            continue;
        }

        index = parse_paragraph_like(parser, region, index, context);
    }
}

/// 인용 마커는 `>` 하나뿐이다. 뒤따르는 공백은 마커가 아니라 들여쓰기 한 단계다
/// (렌더확정: `> 내용`은 `<blockquote><div class='wiki-indent'>…`, `>내용`은 들여쓰기가 없다).
fn quote_marker_length(line: &str) -> Option<usize> {
    line.starts_with('>').then_some(1)
}

/// 들여쓴 줄은 공백뿐이어도 같은 영역이다 — 리스트는 빈 줄에서 끊기지 않는다.
fn indent_marker_length(line: &str) -> Option<usize> {
    line.starts_with(' ').then_some(1)
}

/// 아직 `||`로 닫히지 않은 표 행이 영역을 열어 두는가.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenRow {
    /// 마커 없는 줄에서 영역이 끊긴다.
    Breaks,
    /// 행이 닫힐 때까지 마커 없는 줄도 이 영역의 것이다.
    Holds,
}

/// 줄머리 마커(`>`·들여쓰기)로 열린 영역의 라인을 수집한다.
///
/// 마커가 붙은 줄에서 `{{{` 그룹이 열리면, 그룹이 닫힐 때까지는 마커 없는 줄도
/// 같은 영역에 포함한다. 나무위키가 인용문·리스트 안의 여러 줄 그룹을 이렇게 해석한다.
/// 끝까지 닫히지 않으면 마커 규칙만으로 다시 수집한다.
fn collect_marked_lines(
    region: &Region,
    index: &mut usize,
    marker_length: fn(&str) -> Option<usize>,
) -> Vec<usize> {
    collect_marked_lines_with(region, index, marker_length, OpenRow::Breaks)
}

/// `start`에서 열린 표 행이 `||`로 닫히는 줄. 영역 끝까지 닫히지 않으면 `None`이다.
/// 마커로 옮긴 줄머리든 남은 들여쓰기든 행 판정에서는 무시한다.
fn row_completion_line(
    region: &Region,
    start: usize,
    first_content: &str,
    marker_length: fn(&str) -> Option<usize>,
) -> Option<usize> {
    let mut row = first_content.to_string();
    for line_index in start + 1..region.line_count() {
        let line = region.line_text(line_index);
        let content = match marker_length(line) {
            Some(length) => &line[length..],
            None => line,
        };
        row.push('\n');
        row.push_str(content.trim_start());
        if text::is_row_complete(&row) {
            return Some(line_index);
        }
    }
    None
}

fn collect_marked_lines_with(
    region: &Region,
    index: &mut usize,
    marker_length: fn(&str) -> Option<usize>,
    open_row_policy: OpenRow,
) -> Vec<usize> {
    let start = *index;
    let mut consumed = Vec::new();
    let mut depth = 0;
    // 표 행이 이어지는 마지막 줄. 닫히는 자리를 미리 확인한 행만 붙든다 — 끝내 닫히지
    // 않는 행(`}}}||}}}}}}`처럼 행의 `||` 뒤로 그룹이 닫히는 줄)을 붙들면 영역이 문서
    // 끝까지 밀렸다가 되감기에 걸려 멀쩡한 수집까지 함께 버려진다.
    let mut row_end: Option<usize> = None;
    while *index < region.line_count() {
        let line = region.line_text(*index);
        let marker = marker_length(line);
        let content = match marker {
            Some(length) => &line[length..],
            None if depth > 0 || row_end.is_some_and(|end| *index <= end) => line,
            None => break,
        };
        consumed.push(marker.unwrap_or(0));
        let inside_group = depth > 0;
        depth = (depth + text::brace_delta(content)).max(0);
        // 이미 그룹이 열린 자리의 `||`는 그룹 내용이라 이 영역의 행이 아니다.
        // 행 판정은 더 들어간 들여쓰기를 무시한다 — 표 행은 제 들여쓰기보다 얕은 줄로도
        // 이어지므로(렌더확정: 2칸 들여쓴 `||<colbgcolor=…> 문법 ||{{{#!folding …` 행이
        // 안 들여쓴 `||`에서 닫힌다) 이 영역이 그 줄까지 물고 있어야 한다.
        let row_line = content.trim_start();
        if open_row_policy == OpenRow::Holds
            && !inside_group
            && row_end.is_none_or(|end| *index >= end)
            && text::is_table_start(row_line)
            && !text::is_row_complete(row_line)
        {
            row_end = row_completion_line(region, *index, row_line, marker_length);
        }
        *index += 1;
    }
    if depth > 0 {
        *index = start;
        consumed.clear();
        while *index < region.line_count() {
            let Some(length) = marker_length(region.line_text(*index)) else {
                break;
            };
            consumed.push(length);
            *index += 1;
        }
    }
    consumed
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

/// 주석 줄: 줄머리 `##` 표식과 내용을 나눠 방출한다.
fn emit_comment_line(parser: &mut Parser<'_>, region: &Region, index: usize) {
    let comment = parser.start_node();
    emit_line_prefix(parser, region, index);
    let content = region.lines[index].content.clone();
    parser.emit_token(SyntaxKind::Marker, content.start + 2);
    parser.emit_token(SyntaxKind::Text, content.end);
    comment.complete(parser, SyntaxKind::Comment);
    emit_line_newline(parser, region, index);
}

/// 넘겨주기 줄: `#redirect `/`#넘겨주기 ` 지시자와 대상을 나눠 방출한다.
fn emit_redirect_line(parser: &mut Parser<'_>, region: &Region, index: usize) {
    let node = parser.start_node();
    emit_line_prefix(parser, region, index);
    let content = region.lines[index].content.clone();
    let directive_length = if region.line_text(index).starts_with("#redirect ") {
        "#redirect ".len()
    } else {
        "#넘겨주기 ".len()
    };
    parser.emit_token(SyntaxKind::Directive, content.start + directive_length);
    parser.emit_token(SyntaxKind::Text, content.end);
    node.complete(parser, SyntaxKind::Redirect);
    emit_line_newline(parser, region, index);
}

/// 빈 줄 뒤에 (빈 줄을 건너뛰고) 문단으로 이어질 줄이 있는가.
fn paragraph_follows(region: &Region, index: usize, context: RegionContext) -> bool {
    let mut next = index;
    while next < region.line_count() && region.line_text(next).trim().is_empty() {
        next += 1;
    }
    next < region.line_count() && !is_block_boundary(region.line_text(next), context)
}

/// 문단이 여기서 끝나는가.
///
/// 빈 줄은 경계가 아니다 — 나무위키는 빈 줄로 문단을 나누지 않고 줄바꿈 둘로 본다.
fn is_block_boundary(line: &str, context: RegionContext) -> bool {
    line.starts_with('>')
        || line.starts_with(' ')
        || text::parse_redirect(line).is_some()
        || text::heading_shape(line).is_some()
        || text::is_horizontal_rule(line)
        || text::is_table_start(line)
        || (context.opens_lists() && text::list_marker(line).is_some())
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
            // 다음 리스트 마커까지가 한 덩어리다. 단 `{{{` 그룹이 열려 있으면 그 안의
            // 마커는 마커가 아니라 글자다 — 그룹이 닫힐 때까지 이어 붙인다.
            let start = index;
            let mut depth = 0;
            while index < region.line_count() {
                let line = region.line_text(index);
                if depth == 0 && text::list_marker(line).is_some() {
                    break;
                }
                depth = (depth + text::brace_delta(line)).max(0);
                index += 1;
            }
            let chunk = region.slice_lines(parser.source(), start..index);
            // 빈 줄뿐인 자리는 들여쓰기를 한 겹 더 만들지 않는다 — 그 자리의 빈 문단일 뿐이다.
            if (start..index).all(|line| region.line_text(line).trim().is_empty()) {
                parse_region_blocks(parser, &chunk, RegionContext::Indented);
                continue;
            }
            let marker = parser.start_node();
            parse_region_blocks(parser, &chunk, RegionContext::Indented);
            marker.complete(parser, SyntaxKind::Indent);
        }
    }
}

/// 리스트 항목 줄의 내용이 `{{{` 그룹을 열었다면 그 그룹이 닫히는 줄의 다음 인덱스를 준다.
/// 그룹이 열리지 않았거나 영역 끝까지 닫히지 않으면 `None`이다.
fn closing_line_of_open_group(region: &Region, index: usize, content: &str) -> Option<usize> {
    let mut depth = text::brace_delta(content);
    if depth <= 0 {
        return None;
    }
    let mut end = index + 1;
    while end < region.line_count() {
        depth += text::brace_delta(region.line_text(end));
        end += 1;
        if depth <= 0 {
            return Some(end);
        }
    }
    None
}

/// 리스트 항목 하나가 차지하는 마지막 줄의 다음 인덱스.
///
/// 항목 줄 다음에 마커도 들여쓰기도 없는 줄이 오면 그 항목의 문단이 이어지는 것이다
/// (렌더확정: ` * {{{…}}}\n #설명` → `<li><div class='wiki-paragraph'><code>…</code><br>#설명</div></li>`).
fn item_content_end(region: &Region, index: usize, group_end: Option<usize>) -> usize {
    let mut end = group_end.unwrap_or(index + 1);
    loop {
        end = item_content_run_end(region, end);
        // 뒤따르는 빈 줄도 이 항목의 것이다 — 그 개행이 항목 문단 끝의 `<br>`가 된다
        // (렌더확정: ` A. list 2\n \n A. list 3` → `<li><div class='wiki-paragraph'>list 2<br></div></li>`).
        // 단 뒤에 아무 내용도 없으면 바깥의 것이고, 더 들여쓴 빈 줄은 속내용의 것이다.
        let mut blanks = end;
        while blanks < region.line_count() && is_bare_blank(region.line_text(blanks)) {
            blanks += 1;
        }
        if blanks == end || blanks >= region.line_count() {
            return end;
        }
        // 빈 줄 뒤로도 마커 없는 줄이 이어지면 그것 역시 이 항목의 문단이다 — 빈 줄은
        // 항목을 끊지 않고 그 문단의 첫 `<br>`가 된다(렌더확정: 항목 안 표 뒤의 빈 줄과
        // 뒤따르는 줄이 the seed에서 `<li>…</table></div><div class='wiki-paragraph'><br>위의 …`).
        let next = region.line_text(blanks);
        if next.starts_with(' ') || text::list_marker(next).is_some() {
            return blanks;
        }
        end = blanks;
    }
}

/// 마커도 들여쓰기도 없이 이어지는 줄들의 끝. `{{{` 그룹이 열리거나 표 행이 아직 닫히지
/// 않았으면 그것이 닫힐 때까지는 무엇도 경계가 아니다 — 그 안의 들여쓰기도 빈 줄도 마커도
/// 글자일 뿐이다(렌더확정: 항목 속 `||<colbgcolor=…> 문법 ||{{{#!folding …` 행이 리스트
/// 줄들을 지나 19줄 뒤 `||`에서 닫히는데 the seed는 그 리스트를 셀 안에 넣는다).
fn item_content_run_end(region: &Region, start: usize) -> usize {
    let mut end = start;
    let mut depth = 0;
    let mut row_end: Option<usize> = None;
    while end < region.line_count() {
        let line = region.line_text(end);
        let row_line = line.trim_start();
        if depth == 0
            && row_end.is_none_or(|last| end > last)
            && text::is_table_start(row_line)
            && !text::is_row_complete(row_line)
        {
            row_end = row_completion_line(region, end, row_line, |_| None);
        }
        if depth == 0
            && row_end.is_none_or(|last| end > last)
            && (line.starts_with(' ')
                || line.trim().is_empty()
                || text::list_marker(line).is_some())
        {
            break;
        }
        depth = (depth + text::brace_delta(line)).max(0);
        end += 1;
    }
    end
}

/// 항목 줄 다음의 속내용(더 들여쓴 줄)과 그 뒤 빈 줄을 방출하고 다음 인덱스를 준다.
fn emit_item_continuation(parser: &mut Parser<'_>, region: &Region, start: usize) -> usize {
    let index = emit_item_body(parser, region, start);
    emit_item_trailing_blanks(parser, region, index)
}

/// 항목의 몸통 — 더 들여쓴 속내용(서브리스트 등)과 항목 레벨 연속 문단이 번갈아 올 수 있다.
fn emit_item_body(parser: &mut Parser<'_>, region: &Region, start: usize) -> usize {
    let mut index = start;
    loop {
        let after_content = emit_item_content(parser, region, index);
        let advanced = emit_item_base_continuation(parser, region, after_content);
        if advanced == index {
            return index;
        }
        index = advanced;
    }
}

/// 항목 마커와 같은 들여쓰기(영역 기준 안 들여쓴 줄)의 마커 없는 줄은 그 항목의 연속
/// 문단이다 — `parse_list`는 항상 마커가 줄머리에 오도록 걷힌 영역에서 도므로, 여기서
/// 안 들여쓴 줄은 원래 항목의 들여쓰기만큼 들여썼던 줄, 즉 항목 레벨의 연속이다(렌더확정:
/// 엔하계 위키의 ` * 특징적 표현` 서브리스트 뒤 항목 레벨 ` 이에 대한 …`이 the seed에서
/// 그 `<li>` 안 문단이다). 빈 줄·형제 마커·`{{{` 그룹 경계에서 멈춘다.
fn emit_item_base_continuation(parser: &mut Parser<'_>, region: &Region, start: usize) -> usize {
    let mut index = start;
    let mut depth = 0;
    while index < region.line_count() {
        let line = region.line_text(index);
        if depth == 0
            && (line.trim().is_empty() || line.starts_with(' ') || text::list_marker(line).is_some())
        {
            break;
        }
        depth = (depth + text::brace_delta(line)).max(0);
        index += 1;
    }
    if start < index {
        let sub = region.slice_lines(parser.source(), start..index);
        parse_region_blocks(parser, &sub, RegionContext::ListItem);
    }
    index
}

/// 항목 줄 다음의 속내용(더 들여쓴 줄).
fn emit_item_content(parser: &mut Parser<'_>, region: &Region, start: usize) -> usize {
    let mut index = start;
    // 더 깊이 들여쓴 줄은 이 항목의 속내용이다. 공백뿐인 줄도 들여쓰기가 있으면
    // 거기 속하고(빈 줄이 리스트를 끊지 않는다), 들여쓴 줄에서 `{{{` 그룹이 열리면
    // 닫힐 때까지 들여쓰기 없는 줄도 같이 온다 — 그래야 표가 안 쪼개진다.
    let consumed =
        collect_marked_lines_with(region, &mut index, indent_marker_length, OpenRow::Holds);
    if start < index {
        let sub = region.sub_region(
            parser.source(),
            start..index,
            &consumed,
            SyntaxKind::IndentMarker,
        );
        // 더 들어간 줄은 안쪽 들여쓰기 분기가 알아서 감싼다 — 여기서 볼 것은
        // 이 자리(항목 바로 밑)에 바로 오는 내용뿐이다.
        let first_content =
            (0..sub.line_count()).find(|&line| !sub.line_text(line).trim().is_empty());
        let handled_inside = first_content.is_none_or(|line| {
            let text = sub.line_text(line);
            text.starts_with(' ') || text::list_marker(text).is_some()
        });
        match first_content {
            // 내용 앞의 빈 줄은 제 들여쓰기 단계에 남는다 — 리스트 마커는 제 한 칸을
            // 가져가지만 빈 줄엔 마커가 없어 그 한 칸이 곧 들여쓰기다(렌더확정: the seed가
            // `<div class='wiki-indent'><div class='wiki-paragraph'></div></div><ul class='wiki-list'>`).
            Some(line) if line > 0 && handled_inside => {
                emit_indented_region(parser, &sub, 0..line);
                let rest = sub.slice_lines(parser.source(), line..sub.line_count());
                parse_region_blocks(parser, &rest, RegionContext::ListItem);
            }
            _ if handled_inside => parse_region_blocks(parser, &sub, RegionContext::ListItem),
            _ => emit_indented_region(parser, &sub, 0..sub.line_count()),
        }
    }
    index
}

/// 영역의 일부를 들여쓰기 한 단계로 감싸 방출한다.
fn emit_indented_region(parser: &mut Parser<'_>, region: &Region, lines: Range<usize>) {
    let sub = region.slice_lines(parser.source(), lines);
    let marker = parser.start_node();
    parse_region_blocks(parser, &sub, RegionContext::ListItem);
    marker.complete(parser, SyntaxKind::Indent);
}

/// 속내용 뒤에 오는 빈 줄은 이 항목의 빈 문단이 된다(렌더확정: the seed는
/// `</ol><div class='wiki-paragraph'></div></li>`로 낸다).
fn emit_item_trailing_blanks(parser: &mut Parser<'_>, region: &Region, start: usize) -> usize {
    let mut index = start;
    while index < region.line_count() && region.line_text(index).trim().is_empty() {
        let paragraph = parser.start_node();
        emit_line_prefix(parser, region, index);
        parser.emit_token(SyntaxKind::Text, region.lines[index].content.end);
        paragraph.complete(parser, SyntaxKind::Paragraph);
        emit_line_newline(parser, region, index);
        index += 1;
    }
    index
}

/// 들여쓰기 없이 빈 줄인가. 들여쓴 빈 줄은 그 깊이의 것이라 여기 속하지 않는다.
fn is_bare_blank(line: &str) -> bool {
    line.trim().is_empty() && !line.starts_with(' ')
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
        let marker_length = line_text.len() - content.len();

        // 항목 줄에서 열린 `{{{` 그룹은 닫히는 줄까지가 이 항목의 내용이고, 그 뒤로
        // 마커도 들여쓰기도 없는 줄이 이어지면 그것도 이 항목의 문단이다.
        let end = item_content_end(
            region,
            index,
            closing_line_of_open_group(region, index, content),
        );
        if end > index + 1 {
            let mut consumed = vec![0; end - index];
            consumed[0] = marker_length;
            let sub = region.sub_region(
                parser.source(),
                index..end,
                &consumed,
                SyntaxKind::ListMarker,
            );
            parse_region_blocks(parser, &sub, RegionContext::ListItem);
            // 여러 줄 항목 뒤에도 더 들여쓴 속내용·항목 레벨 연속이 이어질 수 있다 — 한 줄
            // 항목과 같다. 빈 줄 흡수는 하지 않는다(그 빈 줄은 이미 `item_content_end`가 갈랐다).
            index = emit_item_body(parser, region, end);
            item_marker.complete(parser, SyntaxKind::ListItem);
            continue;
        }

        emit_line_prefix(parser, region, index);
        let content_range = region.lines[index].content.clone();
        let marker_end = content_range.start + marker_length;
        parser.emit_token(SyntaxKind::ListMarker, marker_end);
        if !content.is_empty() {
            let paragraph = parser.start_node();
            inline::parse_inline_range(parser, marker_end..content_range.end);
            paragraph.complete(parser, SyntaxKind::Paragraph);
        }
        emit_line_newline(parser, region, index);
        index += 1;

        index = emit_item_continuation(parser, region, index);
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
    context: RegionContext,
) -> usize {
    // 문단 중간에서 열린 `{{{` 그룹은 닫힐 때까지 경계를 무시하고 이어 붙인다.
    // 그룹이 열렸는지는 브레이스 수가 아니라 짝 맞춤으로 본다 — 리터럴이 머금은
    // `{{{`는 그룹을 여는 것이 아니기 때문이다.
    // 짝이 맞는 `{{{` 그룹은 통째로 건너뛴다 — 그 안의 줄은 경계가 아니다. 브레이스
    // 수를 세면 리터럴이 머금은 `{{{`까지 그룹으로 오해하므로 짝 맞춤으로 본다.
    let joined = &region.joined;
    let mut position = region.joined_range_of_lines(start..start + 1).start;
    let mut end = start + 1;
    while position < joined.len() {
        if joined[position..].starts_with("{{{")
            && let Some(close) = text::find_matching_braces(&joined[position..])
        {
            position += close + 3;
            end = end.max(region.locate(position.min(joined.len() - 1)).0 + 1);
            continue;
        }
        if joined.as_bytes()[position] == b'\n' {
            let line = region.locate(position).0 + 1;
            if line >= region.line_count() {
                break;
            }
            if line >= end && is_block_boundary(region.line_text(line), context) {
                end = line;
                break;
            }
            end = end.max(line + 1);
        }
        position += next_char_length(joined, position);
    }
    let end = end.min(region.line_count());
    emit_paragraph_segments(parser, region, start..end);
    emit_line_newline(parser, region, end - 1);
    end
}

// 문단 하나를 열고 그 안에 텍스트와 `{{{` 그룹을 교대로 넣는다.
//
// 나무위키에서 `{{{` 그룹(`#!wiki`·`#!folding`·`#!syntax`·`#!if` …)은 블록이 아니라
// **인라인 요소**다 — 렌더 결과에서 이들의 부모는 언제나 문단이고, 홀로 선 그룹도
// 그것만 든 문단 안에 들어간다.
fn emit_paragraph_segments(parser: &mut Parser<'_>, region: &Region, lines: Range<usize>) {
    let paragraph = parser.start_node();
    // 주석 줄은 개행까지 통째로 사라진다 — 문단을 끊지도, 줄바꿈을 남기지도 않는다
    // (렌더확정: `가나다라\n## 주석\n마바사아` → `가나다라<br>마바사아`).
    let mut start = lines.start;
    for index in lines.clone() {
        if !is_comment_line(region, start..index) {
            continue;
        }
        if let Some(last) = last_content_line(region, start..index) {
            emit_paragraph_content(
                parser,
                region,
                region.joined_range_of_lines(start..last + 1),
            );
            // 주석 뒤로 내용이 이어지면 주석을 지운 자리에서 두 줄이 맞붙는다 —
            // 그 사이 개행 하나가 줄바꿈으로 남는다.
            if last_content_line(region, index + 1..lines.end).is_some() {
                emit_line_newline(parser, region, index - 1);
            } else {
                emit_trailing_newlines(parser, region, last..index - 1);
            }
        }
        let comment = parser.start_node();
        emit_line_prefix(parser, region, index);
        parser.emit_token(SyntaxKind::Marker, region.lines[index].content.start + 2);
        parser.emit_token(SyntaxKind::Text, region.lines[index].content.end);
        emit_line_newline(parser, region, index);
        comment.complete(parser, SyntaxKind::Comment);
        start = index + 1;
    }
    if let Some(last) = last_content_line(region, start..lines.end) {
        emit_paragraph_content(
            parser,
            region,
            region.joined_range_of_lines(start..last + 1),
        );
        emit_trailing_newlines(parser, region, last..lines.end - 1);
    }
    paragraph.complete(parser, SyntaxKind::Paragraph);
}

/// 이 범위에서 비어 있지 않은 마지막 줄.
fn last_content_line(region: &Region, lines: Range<usize>) -> Option<usize> {
    lines
        .rev()
        .find(|&index| !region.line_text(index).trim().is_empty())
}

/// 내용 뒤 빈 줄들이 남기는 줄바꿈. 마지막 개행 하나는 바깥이 가져간다
/// (the seed: `내용.\n\n== 다음 ==` → `<div class='wiki-paragraph'>내용.<br></div>`).
fn emit_trailing_newlines(parser: &mut Parser<'_>, region: &Region, lines: Range<usize>) {
    for index in lines {
        emit_line_newline(parser, region, index);
    }
}

/// `{{{` 그룹 안의 `##`은 주석이 아니라 그 그룹의 글자다. `run`은 이 줄 앞의 같은 문단 범위다.
fn is_comment_line(region: &Region, run: Range<usize>) -> bool {
    is_comment(region, run.end)
        && (run.is_empty()
            || !text::has_open_group(&region.joined[region.joined_range_of_lines(run)]))
}

/// `##`은 줄머리에 있어야 주석이다. 들여쓴 ` ## …`은 글자다(렌더확정: the seed가
/// `<div class='wiki-indent'><div class='wiki-paragraph'>## 주석 앞 띄어쓰기</div></div>`로 낸다).
fn is_comment(region: &Region, index: usize) -> bool {
    region.lines[index].prefix.is_empty() && region.line_text(index).starts_with("##")
}

fn emit_paragraph_content(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    let joined = &region.joined;
    let mut position = joined_range.start;
    let mut segment_start = joined_range.start;
    while position < joined_range.end {
        if !parser.tick() {
            break;
        }
        // `\`는 다음 글자를 글자로 만든다. 여기서 건너뛰지 않으면 `\{{{`의 `{{{`를
        // 그룹으로 떼어 내 인라인 파서가 이스케이프를 볼 기회를 잃는다.
        if joined[position..joined_range.end].starts_with('\\') {
            position += 1;
            position += next_char_length(joined, position).min(joined_range.end - position);
            continue;
        }
        // 각주와 링크는 그룹보다 바깥이다. 안쪽 그룹을 먼저 떼어 내면 이들이 끊긴다
        // (`본문[* 앞{{{#!wiki …\n뒤}}}]`, `[[:분류:X|{{{#!wiki style="…"\n글}}}]]`).
        let rest = &joined[position..joined_range.end];
        if rest.starts_with("[*")
            && let Some(close) = text::find_matching_bracket(rest)
        {
            position += close + 1;
            continue;
        }
        if rest.starts_with("[[")
            && let Some(close) = text::find_matching_double_bracket(rest)
        {
            position += close + 2;
            continue;
        }
        if joined[position..joined_range.end].starts_with("{{{") {
            let group_source = &joined[position..joined_range.end];
            if let Some(close) = text::find_matching_braces(group_source) {
                let group_end = position + close + 3;
                if group_source[..close + 3].contains('\n') {
                    emit_inline_segment(parser, region, segment_start..position);
                    brace::parse_brace_group(parser, region, position..group_end, true);
                    position = group_end;
                    segment_start = group_end;
                    continue;
                }
                // 한 줄에서 닫힌 그룹은 인라인 파서가 다룬다.
                position = group_end;
                continue;
            }
            position += 3;
            continue;
        }
        position += next_char_length(joined, position);
    }
    emit_inline_segment(parser, region, segment_start..joined_range.end);
}

/// 문단 안의 텍스트 조각. 문단 노드는 호출부가 이미 열어 두었다.
fn emit_inline_segment(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    if joined_range.is_empty() {
        return;
    }
    emit_flowing_inline(parser, region, joined_range);
}

/// 라인 조각마다 인라인 파싱을 수행하고 전환부는 개행+prefix로 방출한다.
fn emit_flowing_inline(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    if joined_range.is_empty() {
        return;
    }
    emit_flowing_lines(parser, region, joined_range.clone());
    // 범위가 줄머리에서 끝나면 마지막 개행은 하위 영역이 갖지 않는다. 여기서 방출하지
    // 않으면 뒤따르는 `{{{` 그룹의 마커가 그 개행까지 머금어 줄바꿈이 사라진다.
    let (line, column) = region.locate(joined_range.end);
    if column == 0 && line > 0 && parser.position() < region.lines[line - 1].newline.end {
        emit_line_newline(parser, region, line - 1);
    }
}

fn emit_flowing_lines(parser: &mut Parser<'_>, region: &Region, joined_range: Range<usize>) {
    let sub = region.sub_region_from_joined(parser.source(), joined_range);
    // 줄머리에 소비할 마커가 없으면 원문에서 연속이므로 통째로 인라인 파싱한다.
    // 각주나 `{{{` 그룹이 줄을 넘을 수 있기 때문이다(`[* 앞{{{#!wiki …\n뒤}}}]`).
    if let Some(range) = contiguous_content(&sub) {
        inline::parse_inline_range(parser, range);
        return;
    }
    for index in 0..sub.line_count() {
        emit_line_prefix(parser, &sub, index);
        inline::parse_inline_range(parser, sub.lines[index].content.clone());
        emit_line_newline(parser, &sub, index);
    }
}

/// 모든 줄이 마커 없이 원문에서 이어져 있으면 그 전체 범위.
fn contiguous_content(region: &Region) -> Option<Range<usize>> {
    let first = region.lines.first()?;
    let last = region.lines.last()?;
    if !region.lines.iter().all(|line| line.prefix.is_empty()) {
        return None;
    }
    // 줄 사이가 개행 하나로만 이어져야 한다.
    for window in region.lines.windows(2) {
        if window[0].newline.end != window[1].content.start {
            return None;
        }
    }
    Some(first.content.start..last.newline.end)
}

fn next_char_length(text: &str, position: usize) -> usize {
    text[position..]
        .chars()
        .next()
        .map(char::len_utf8)
        .unwrap_or(1)
}
