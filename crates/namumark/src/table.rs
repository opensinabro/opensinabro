use crate::ast::{
    HorizontalAlignment, Table, TableAttribute, TableAttributeScope, TableCell, TableRow,
    VerticalAlignment,
};
use crate::block::{brace_delta, parse_blocks};
use crate::inline::parse_inlines;

pub(crate) fn is_table_start(line: &str) -> bool {
    line.starts_with("||") || (line.starts_with('|') && line[1..].contains('|'))
}

pub(crate) fn parse_table(lines: &[&str]) -> Option<(Table, usize)> {
    let (caption, first_row_line) = extract_caption(lines[0]);
    if !first_row_line.starts_with("||") {
        return None;
    }

    let mut row_sources: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let mut row_source = if index == 0 {
            first_row_line.clone()
        } else {
            if !lines[index].starts_with("||") {
                break;
            }
            lines[index].to_string()
        };
        index += 1;
        while !is_row_complete(&row_source) && index < lines.len() {
            row_source.push('\n');
            row_source.push_str(lines[index]);
            index += 1;
        }
        if !is_row_complete(&row_source) && row_sources.is_empty() {
            return None;
        }
        row_sources.push(row_source);
    }

    let rows: Vec<TableRow> = row_sources
        .iter()
        .map(|source| TableRow {
            cells: split_cells(source.trim_end())
                .into_iter()
                .map(|(span_pairs, content)| parse_cell(span_pairs, content))
                .collect(),
        })
        .filter(|row| !row.cells.is_empty())
        .collect();
    if rows.is_empty() {
        return None;
    }

    let caption = caption.map(|caption| parse_inlines(&caption));
    Some((Table { caption, rows }, index))
}

// 표 첫 줄이 `|캡션|`으로 시작하면 캡션을 분리하고 그 자리를 `||`로 대체한다.
fn extract_caption(line: &str) -> (Option<String>, String) {
    if !line.starts_with('|') || line.starts_with("||") {
        return (None, line.to_string());
    }
    let rest = &line[1..];
    let Some(end) = rest.find('|') else {
        return (None, line.to_string());
    };
    let caption = rest[..end].to_string();
    (Some(caption), format!("||{}", &rest[end + 1..]))
}

fn is_row_complete(row_source: &str) -> bool {
    if brace_delta(row_source) > 0 {
        return false;
    }
    let trimmed = row_source.trim_end();
    let without_pipes = trimmed.trim_end_matches('|');
    if without_pipes.is_empty() {
        // 파이프만 있는 행은 여는 `||`와 닫는 `||`가 모두 있어야 완결이다.
        return trimmed.len() >= 4;
    }
    trimmed.ends_with("||")
}

// 각 셀을 (선행 `||` 쌍 개수, 내용)으로 분리한다. 쌍 개수가 자동 colspan이 된다.
fn split_cells(row_source: &str) -> Vec<(usize, &str)> {
    let bytes = row_source.as_bytes();
    let mut cells = Vec::new();
    let mut span_pairs = pipe_run_length(bytes, 0) / 2;
    let mut position = span_pairs * 2;
    let mut cell_start = position;
    let mut depth = 0usize;
    while position < bytes.len() {
        if bytes[position..].starts_with(b"{{{") {
            depth += 1;
            position += 3;
        } else if bytes[position..].starts_with(b"}}}") {
            depth = depth.saturating_sub(1);
            position += 3;
        } else if depth == 0 && bytes[position] == b'|' {
            let run = pipe_run_length(bytes, position);
            if run >= 2 {
                cells.push((span_pairs, &row_source[cell_start..position]));
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
        let trailing = row_source[cell_start..].trim_end();
        if !trailing.is_empty() {
            cells.push((span_pairs, trailing));
        }
    }
    cells
}

fn pipe_run_length(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .take_while(|&&byte| byte == b'|')
        .count()
}

struct CellBuilder {
    column_span: Option<u32>,
    row_span: u32,
    horizontal_alignment: Option<HorizontalAlignment>,
    vertical_alignment: Option<VerticalAlignment>,
    attributes: Vec<TableAttribute>,
}

fn parse_cell(span_pairs: usize, source: &str) -> TableCell {
    let mut builder = CellBuilder {
        column_span: None,
        row_span: 1,
        horizontal_alignment: None,
        vertical_alignment: None,
        attributes: Vec::new(),
    };

    let mut rest = source;
    while let Some(after_open) = rest.strip_prefix('<') {
        let Some(close) = after_open.find('>') else {
            break;
        };
        let token = &after_open[..close];
        if token.is_empty() || token.contains('<') || !builder.apply_option(token) {
            break;
        }
        rest = &after_open[close + 1..];
    }

    let explicit_alignment = builder.horizontal_alignment.is_some();
    let mut content = rest;
    if explicit_alignment {
        content = content.strip_prefix(' ').unwrap_or(content);
        content = content.strip_suffix(' ').unwrap_or(content);
    } else if let Some(stripped) = content.strip_prefix(' ') {
        if let Some(both) = stripped.strip_suffix(' ') {
            builder.horizontal_alignment = Some(HorizontalAlignment::Center);
            content = both;
        } else {
            builder.horizontal_alignment = Some(HorizontalAlignment::Right);
            content = stripped;
        }
    } else {
        content = content.strip_suffix(' ').unwrap_or(content);
    }

    let content_lines: Vec<&str> = content.lines().collect();
    TableCell {
        column_span: builder.column_span.unwrap_or(span_pairs as u32).max(1),
        row_span: builder.row_span,
        horizontal_alignment: builder
            .horizontal_alignment
            .unwrap_or(HorizontalAlignment::Left),
        vertical_alignment: builder.vertical_alignment,
        attributes: builder.attributes,
        blocks: parse_blocks(&content_lines, false),
    }
}

impl CellBuilder {
    fn apply_option(&mut self, token: &str) -> bool {
        match token {
            "(" => {
                self.horizontal_alignment = Some(HorizontalAlignment::Left);
                return true;
            }
            ":" => {
                self.horizontal_alignment = Some(HorizontalAlignment::Center);
                return true;
            }
            ")" => {
                self.horizontal_alignment = Some(HorizontalAlignment::Right);
                return true;
            }
            "keepall" => return self.push_flag(TableAttributeScope::Cell, "keepall"),
            "nopad" => return self.push_flag(TableAttributeScope::Cell, "nopad"),
            "rowkeepall" => return self.push_flag(TableAttributeScope::Row, "keepall"),
            "colkeepall" => return self.push_flag(TableAttributeScope::Column, "keepall"),
            _ => {}
        }

        if let Some(number) = token.strip_prefix('-')
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(value) = number.parse::<u32>()
        {
            self.column_span = Some(value.max(1));
            return true;
        }

        let (vertical_alignment, rowspan_source) = if let Some(rest) = token.strip_prefix('^') {
            (Some(VerticalAlignment::Top), rest)
        } else if let Some(rest) = token.strip_prefix('v') {
            (Some(VerticalAlignment::Bottom), rest)
        } else {
            (None, token)
        };
        if let Some(number) = rowspan_source.strip_prefix('|')
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(value) = number.parse::<u32>()
        {
            self.row_span = value.max(1);
            if vertical_alignment.is_some() {
                self.vertical_alignment = vertical_alignment;
            }
            return true;
        }

        if let Some((name, value)) = token.split_once('=') {
            let normalized = name.replace(' ', "").to_ascii_lowercase();
            let Some((scope, canonical)) = resolve_attribute_name(&normalized) else {
                return false;
            };
            self.attributes.push(TableAttribute {
                scope,
                name: canonical.to_string(),
                value: Some(value.to_string()),
            });
            return true;
        }

        if is_bare_color(token) {
            self.attributes.push(TableAttribute {
                scope: TableAttributeScope::Cell,
                name: "bgcolor".to_string(),
                value: Some(token.to_string()),
            });
            return true;
        }

        false
    }

    fn push_flag(&mut self, scope: TableAttributeScope, name: &str) -> bool {
        self.attributes.push(TableAttribute {
            scope,
            name: name.to_string(),
            value: None,
        });
        true
    }
}

fn resolve_attribute_name(name: &str) -> Option<(TableAttributeScope, &str)> {
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

    if let Some(rest) = name.strip_prefix("table")
        && TABLE_NAMES.contains(&rest)
    {
        return Some((TableAttributeScope::Table, rest));
    }
    if let Some(rest) = name.strip_prefix("row")
        && ROW_NAMES.contains(&rest)
    {
        return Some((TableAttributeScope::Row, rest));
    }
    if let Some(rest) = name.strip_prefix("col")
        && COLUMN_NAMES.contains(&rest)
    {
        return Some((TableAttributeScope::Column, rest));
    }
    if CELL_NAMES.contains(&name) {
        return Some((TableAttributeScope::Cell, name));
    }
    None
}

fn is_bare_color(token: &str) -> bool {
    if let Some(hex) = token.strip_prefix('#') {
        matches!(hex.len(), 3 | 6) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else {
        !token.is_empty() && token.chars().all(char::is_alphanumeric)
    }
}
