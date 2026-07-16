use crate::ast::{
    Block, CodeBlock, ColoredBlock, Document, Folding, Heading, Inline, List, ListItem, ListKind,
    SizedBlock, WikiStyle,
};
use crate::inline::{find_matching_braces, parse_color_marker, parse_inlines, parse_size_marker};
use crate::table::{is_table_start, parse_table};

pub(crate) fn parse_document(source: &str) -> Document {
    let lines: Vec<&str> = source.lines().collect();
    Document {
        blocks: parse_blocks(&lines, false),
    }
}

// list_context: 들여쓰기가 이미 소비된 영역 안에서는 열 0의 리스트 마커를 인식한다.
pub(crate) fn parse_blocks(lines: &[&str], list_context: bool) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];

        if line.trim().is_empty() {
            index += 1;
            continue;
        }
        if let Some(comment) = line.strip_prefix("##") {
            blocks.push(Block::Comment(comment.to_string()));
            index += 1;
            continue;
        }
        if let Some(target) = parse_redirect(line) {
            blocks.push(Block::Redirect(target));
            index += 1;
            continue;
        }
        if let Some(heading) = parse_heading(line) {
            blocks.push(Block::Heading(heading));
            index += 1;
            continue;
        }
        if is_horizontal_rule(line) {
            blocks.push(Block::HorizontalRule);
            index += 1;
            continue;
        }
        if line.starts_with('>') {
            let mut quote_lines = Vec::new();
            while index < lines.len() {
                let Some(stripped) = lines[index].strip_prefix('>') else {
                    break;
                };
                quote_lines.push(stripped.strip_prefix(' ').unwrap_or(stripped));
                index += 1;
            }
            blocks.push(Block::Quote(parse_blocks(&quote_lines, false)));
            continue;
        }
        if list_context && let Some((kind, _, _)) = split_list_marker(line) {
            let (list, consumed) = parse_list(&lines[index..], kind);
            blocks.push(Block::List(list));
            index += consumed;
            continue;
        }
        if line.starts_with(' ') {
            let mut region = Vec::new();
            while index < lines.len()
                && lines[index].starts_with(' ')
                && !lines[index].trim().is_empty()
            {
                region.push(&lines[index][1..]);
                index += 1;
            }
            if split_list_marker(region[0]).is_some() {
                // 리스트 자체가 들여쓰기를 내포하므로 Indent로 감싸지 않는다.
                // 같은 깊이에 섞인 일반 블록은 들여쓰기 문단으로 취급한다.
                let mut pending_indent: Vec<Block> = Vec::new();
                for block in parse_blocks(&region, true) {
                    if matches!(block, Block::List(_)) {
                        if !pending_indent.is_empty() {
                            blocks.push(Block::Indent(std::mem::take(&mut pending_indent)));
                        }
                        blocks.push(block);
                    } else {
                        pending_indent.push(block);
                    }
                }
                if !pending_indent.is_empty() {
                    blocks.push(Block::Indent(pending_indent));
                }
            } else {
                blocks.push(Block::Indent(parse_blocks(&region, true)));
            }
            continue;
        }
        if is_table_start(line)
            && let Some((table, consumed)) = parse_table(&lines[index..])
        {
            blocks.push(Block::Table(table));
            index += consumed;
            continue;
        }
        // 문단 중간에서 열린 `{{{` 그룹은 닫힐 때까지 경계를 무시하고 이어 붙인다.
        let mut paragraph_lines = vec![line];
        let mut depth = brace_delta(line).max(0);
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let next = lines[cursor];
            if depth == 0 && is_block_boundary(next, list_context) {
                break;
            }
            depth = (depth + brace_delta(next)).max(0);
            paragraph_lines.push(next);
            cursor += 1;
        }
        if depth > 0 {
            // 그룹이 끝내 닫히지 않으면 경계 규칙만으로 다시 수집해
            // 문서 나머지가 한 문단으로 뭉치는 것을 막는다.
            paragraph_lines = vec![line];
            cursor = index + 1;
            while cursor < lines.len() && !is_block_boundary(lines[cursor], list_context) {
                paragraph_lines.push(lines[cursor]);
                cursor += 1;
            }
        }
        index = cursor;
        parse_paragraph_segments(&paragraph_lines.join("\n"), &mut blocks);
    }
    blocks
}

// 문단 텍스트를 스캔해 여러 줄에 걸친 `{{{ ... }}}` 그룹을 블록으로 분리하고,
// 나머지 텍스트는 문단으로 만든다. 한 줄 안에서 닫힌 그룹은 인라인으로 남긴다.
fn parse_paragraph_segments(source: &str, blocks: &mut Vec<Block>) {
    let bytes = source.as_bytes();
    let mut text_start = 0;
    let mut position = 0;
    let push_text = |start: usize, end: usize, blocks: &mut Vec<Block>| {
        // 블록 그룹과 인접한 개행은 구조적 구분이므로 줄바꿈으로 취급하지 않는다.
        let segment = &source[start..end];
        let segment = segment.strip_prefix('\n').unwrap_or(segment);
        let segment = segment.strip_suffix('\n').unwrap_or(segment);
        if !segment.trim().is_empty() {
            let segment_lines: Vec<&str> = segment.lines().collect();
            blocks.push(Block::Paragraph(parse_inline_lines(&segment_lines)));
        }
    };
    while position < bytes.len() {
        if bytes[position..].starts_with(b"{{{") {
            let group_source = &source[position..];
            if let Some(end) = find_matching_braces(group_source) {
                let group = &group_source[..end + 3];
                if group.contains('\n') {
                    push_text(text_start, position, blocks);
                    let group_lines: Vec<&str> = group.lines().collect();
                    if let Some((block, _)) = parse_brace_block(&group_lines) {
                        blocks.push(block);
                    }
                    position += end + 3;
                    text_start = position;
                    continue;
                }
                position += end + 3;
                continue;
            }
            position += 3;
            continue;
        }
        position += 1;
    }
    push_text(text_start, source.len(), blocks);
}

fn parse_inline_lines(lines: &[&str]) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            inlines.push(Inline::LineBreak);
        }
        inlines.extend(parse_inlines(line));
    }
    inlines
}

fn is_block_boundary(line: &str, list_context: bool) -> bool {
    line.trim().is_empty()
        || line.starts_with("##")
        || line.starts_with('>')
        || line.starts_with(' ')
        || line.starts_with("{{{")
        || is_table_start(line)
        || parse_redirect(line).is_some()
        || parse_heading(line).is_some()
        || is_horizontal_rule(line)
        || (list_context && split_list_marker(line).is_some())
}

fn parse_redirect(line: &str) -> Option<String> {
    let target = line
        .strip_prefix("#redirect ")
        .or_else(|| line.strip_prefix("#넘겨주기 "))?;
    Some(target.trim().to_string())
}

fn parse_heading(line: &str) -> Option<Heading> {
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
    Some(Heading {
        level: level as u8,
        folded,
        content: parse_inlines(content),
    })
}

fn is_horizontal_rule(line: &str) -> bool {
    (4..=9).contains(&line.len()) && line.bytes().all(|byte| byte == b'-')
}

// 순서 리스트 마커는 항상 `1.` `a.` 등 리터럴이고 번호는 자동 증가한다.
// 마커 뒤 공백은 선택이며, `1.#42`는 시작 번호를 재지정한다.
fn split_list_marker(line: &str) -> Option<(ListKind, Option<u32>, &str)> {
    if let Some(rest) = line.strip_prefix('*') {
        return Some((ListKind::Unordered, None, strip_single_space(rest)));
    }
    const ORDERED_MARKERS: [(&str, ListKind); 5] = [
        ("1.", ListKind::Decimal),
        ("a.", ListKind::LowerAlphabet),
        ("A.", ListKind::UpperAlphabet),
        ("i.", ListKind::LowerRoman),
        ("I.", ListKind::UpperRoman),
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
        return Some((kind, start_number, strip_single_space(rest)));
    }
    None
}

fn strip_single_space(rest: &str) -> &str {
    rest.strip_prefix(' ').unwrap_or(rest)
}

fn parse_list(lines: &[&str], kind: ListKind) -> (List, usize) {
    let mut items = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some((item_kind, start_number, content)) = split_list_marker(lines[index]) else {
            break;
        };
        if item_kind != kind {
            break;
        }
        index += 1;
        let mut continuation = Vec::new();
        while index < lines.len()
            && lines[index].starts_with(' ')
            && !lines[index].trim().is_empty()
        {
            continuation.push(&lines[index][1..]);
            index += 1;
        }
        let mut item_blocks = Vec::new();
        if !content.is_empty() {
            item_blocks.push(Block::Paragraph(parse_inlines(content)));
        }
        item_blocks.extend(parse_blocks(&continuation, true));
        items.push(ListItem {
            start_number,
            blocks: item_blocks,
        });
    }
    (List { kind, items }, index)
}

fn parse_brace_block(lines: &[&str]) -> Option<(Block, usize)> {
    let mut depth = 0i32;
    let mut end = None;
    for (offset, line) in lines.iter().enumerate() {
        depth += brace_delta(line);
        if depth <= 0 {
            end = Some(offset);
            break;
        }
    }
    let end = end?;
    if end == 0 {
        // 한 줄 안에서 닫힌 경우는 인라인 리터럴로 처리한다.
        return None;
    }
    let header = &lines[0][3..];
    let consumed = end + 1;

    if let Some(rest) = strip_directive(header, "#!syntax") {
        let language = rest.trim();
        let language = (!language.is_empty()).then(|| language.to_string());
        let source = brace_content_lines(lines, end, None).join("\n");
        return Some((Block::CodeBlock(CodeBlock { language, source }), consumed));
    }
    if let Some(rest) = strip_directive(header, "#!wiki") {
        let (style, dark_style, leftover) = parse_wiki_style_attributes(rest);
        let content = brace_content_lines(lines, end, Some(leftover));
        return Some((
            Block::WikiStyle(WikiStyle {
                style,
                dark_style,
                blocks: parse_blocks(&content, false),
            }),
            consumed,
        ));
    }
    if let Some(rest) = strip_directive(header, "#!folding") {
        let summary = parse_inlines(rest.trim());
        let content = brace_content_lines(lines, end, None);
        return Some((
            Block::Folding(Folding {
                summary,
                blocks: parse_blocks(&content, false),
            }),
            consumed,
        ));
    }
    if let Some(rest) = strip_directive(header, "#!html") {
        let content = brace_content_lines(lines, end, Some(rest));
        return Some((Block::Html(content.join("\n")), consumed));
    }
    if let Some((level, rest)) = parse_size_marker(header) {
        let content = brace_content_lines(lines, end, Some(rest));
        return Some((
            Block::Sized(SizedBlock {
                level,
                blocks: parse_blocks(&content, false),
            }),
            consumed,
        ));
    }
    if let Some((color, dark_color, rest)) = parse_color_marker(header) {
        let content = brace_content_lines(lines, end, Some(rest));
        return Some((
            Block::Colored(ColoredBlock {
                color,
                dark_color,
                blocks: parse_blocks(&content, false),
            }),
            consumed,
        ));
    }

    let first_line = (!header.is_empty()).then_some(header);
    let source = brace_content_lines(lines, end, first_line).join("\n");
    Some((
        Block::CodeBlock(CodeBlock {
            language: None,
            source,
        }),
        consumed,
    ))
}

fn strip_directive<'line>(header: &'line str, directive: &str) -> Option<&'line str> {
    let rest = header.strip_prefix(directive)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.strip_prefix(' ').map(str::trim_start)
}

fn brace_content_lines<'lines>(
    lines: &[&'lines str],
    end: usize,
    first_line: Option<&'lines str>,
) -> Vec<&'lines str> {
    let mut content = Vec::new();
    if let Some(first_line) = first_line
        && !first_line.is_empty()
    {
        content.push(first_line);
    }
    content.extend(lines[1..end].iter().copied());
    let closing_line = lines[end].strip_suffix("}}}").unwrap_or(lines[end]);
    if !closing_line.is_empty() {
        content.push(closing_line);
    }
    content
}

// `style="..."`, `dark-style='...'` 속성을 반복 추출한다. 남는 텍스트는 본문 첫 줄로 취급한다.
fn parse_wiki_style_attributes(source: &str) -> (Option<String>, Option<String>, &str) {
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

pub(crate) fn brace_delta(line: &str) -> i32 {
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
