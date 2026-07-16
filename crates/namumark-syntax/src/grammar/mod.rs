//! 문법 계층. 원문을 라인 단위 Region으로 보고 기존 파서의 결정 로직(crate::text)을
//! 재사용하되, 값 대신 이벤트를 방출한다.
//!
//! 모든 오프셋은 원문 전역 바이트 오프셋이다. 결정용 문자열(Region::joined)은
//! 라인 content를 `\n`으로 연결한 것으로, 옛 파서가 보던 입력과 동일하다.

mod block;
mod brace;
mod inline;
mod table;

use crate::kind::SyntaxKind;
use crate::parser::Parser;
use std::ops::Range;

pub(crate) fn document(parser: &mut Parser<'_>) {
    let marker = parser.start_node();
    let region = Region::from_source(parser.source());
    block::parse_region_blocks(parser, &region, false);
    // 방어: 연료 소진 등으로 남은 원문이 있으면 통째로 방출해 무손실을 지킨다.
    let end = parser.source().len();
    if parser.position() < end {
        parser.emit_token(SyntaxKind::Text, end);
    }
    marker.complete(parser, SyntaxKind::Document);
}

/// 물리적 라인 하나. prefix는 상위 구조(인용 `>`, 들여쓰기)가 소비한 앞부분이다.
#[derive(Clone)]
pub(crate) struct Line {
    pub(crate) prefix: Vec<(SyntaxKind, Range<usize>)>,
    pub(crate) content: Range<usize>,
    /// 개행 바이트(`\n` 또는 `\r\n`). 빈 범위면 이 라인의 개행은 이 영역 소유가 아니다.
    pub(crate) newline: Range<usize>,
}

pub(crate) struct Region {
    pub(crate) lines: Vec<Line>,
    /// 결정용: 라인 content를 `\n`으로 연결한 문자열
    pub(crate) joined: String,
    joined_starts: Vec<usize>,
}

impl Region {
    pub(crate) fn from_source(source: &str) -> Region {
        let mut lines = Vec::new();
        let mut start = 0;
        let bytes = source.as_bytes();
        for (index, &byte) in bytes.iter().enumerate() {
            if byte == b'\n' {
                let content_end = if index > start && bytes[index - 1] == b'\r' {
                    index - 1
                } else {
                    index
                };
                lines.push(Line {
                    prefix: Vec::new(),
                    content: start..content_end,
                    newline: content_end..index + 1,
                });
                start = index + 1;
            }
        }
        if start < source.len() {
            lines.push(Line {
                prefix: Vec::new(),
                content: start..source.len(),
                newline: source.len()..source.len(),
            });
        }
        Region::new(source, lines)
    }

    pub(crate) fn new(source: &str, lines: Vec<Line>) -> Region {
        let mut joined = String::new();
        let mut joined_starts = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                joined.push('\n');
            }
            joined_starts.push(joined.len());
            joined.push_str(&source[line.content.clone()]);
        }
        Region {
            lines,
            joined,
            joined_starts,
        }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn line_text(&self, index: usize) -> &str {
        let start = self.joined_starts[index];
        &self.joined[start..start + self.lines[index].content.len()]
    }

    pub(crate) fn joined_start(&self, index: usize) -> usize {
        self.joined_starts[index]
    }

    pub(crate) fn joined_range_of_lines(&self, line_range: Range<usize>) -> Range<usize> {
        let start = self.joined_starts[line_range.start];
        let last = line_range.end - 1;
        let end = self.joined_starts[last] + self.lines[last].content.len();
        start..end
    }

    /// joined 오프셋 → (라인 인덱스, 라인 내 컬럼)
    pub(crate) fn locate(&self, joined_offset: usize) -> (usize, usize) {
        let line_index = self
            .joined_starts
            .partition_point(|&start| start <= joined_offset)
            .saturating_sub(1);
        (line_index, joined_offset - self.joined_starts[line_index])
    }

    pub(crate) fn to_global(&self, joined_offset: usize) -> usize {
        let (line_index, column) = self.locate(joined_offset);
        self.lines[line_index].content.start + column
    }

    /// prefix를 유지한 채 라인 부분집합만 갖는 하위 영역
    pub(crate) fn slice_lines(&self, source: &str, line_range: Range<usize>) -> Region {
        Region::new(source, self.lines[line_range].to_vec())
    }

    /// 각 라인 content 앞의 `consumed[i]` 바이트를 `kind` prefix로 옮긴 하위 영역
    pub(crate) fn sub_region(
        &self,
        source: &str,
        line_range: Range<usize>,
        consumed: &[usize],
        kind: SyntaxKind,
    ) -> Region {
        let lines = self.lines[line_range]
            .iter()
            .zip(consumed)
            .map(|(line, &consume)| {
                let mut prefix = line.prefix.clone();
                let content_start = line.content.start + consume;
                if consume > 0 {
                    prefix.push((kind, line.content.start..content_start));
                }
                Line {
                    prefix,
                    content: content_start..line.content.end,
                    newline: line.newline.clone(),
                }
            })
            .collect();
        Region::new(source, lines)
    }

    /// joined 범위에 해당하는 하위 영역. 중간에서 시작하는 첫 조각은 prefix가 없고,
    /// 마지막 조각의 개행은 소유하지 않는다(바깥 워커가 방출).
    pub(crate) fn sub_region_from_joined(
        &self,
        source: &str,
        joined_range: Range<usize>,
    ) -> Region {
        if joined_range.is_empty() {
            return Region::new(source, Vec::new());
        }
        let (first_line, first_column) = self.locate(joined_range.start);
        let (last_line, last_column) = self.locate(joined_range.end);
        // joined_range.end가 라인 경계('\n' 자리)면 이전 라인의 끝으로 정규화
        let (last_line, last_column) = if last_column == 0 && last_line > first_line {
            (last_line - 1, self.lines[last_line - 1].content.len())
        } else {
            (last_line, last_column)
        };
        let mut lines = Vec::with_capacity(last_line - first_line + 1);
        for index in first_line..=last_line {
            let original = &self.lines[index];
            let content_start = if index == first_line {
                original.content.start + first_column
            } else {
                original.content.start
            };
            let content_end = if index == last_line {
                original.content.start + last_column
            } else {
                original.content.end
            };
            let prefix = if index == first_line && first_column > 0 {
                Vec::new()
            } else {
                original.prefix.clone()
            };
            let newline = if index == last_line {
                content_end..content_end
            } else {
                original.newline.clone()
            };
            lines.push(Line {
                prefix,
                content: content_start..content_end,
                newline,
            });
        }
        Region::new(source, lines)
    }
}

// ---- 방출 헬퍼 ----

// 이미 방출된 prefix는 건너뛰어 멱등으로 동작한다 (여러 방출 경로가 겹칠 수 있다).
pub(crate) fn emit_line_prefix(parser: &mut Parser<'_>, region: &Region, line_index: usize) {
    for (kind, range) in region.lines[line_index].prefix.clone() {
        if range.end > parser.position() {
            parser.emit_token(kind, range.end);
        }
    }
}

pub(crate) fn emit_line_newline(parser: &mut Parser<'_>, region: &Region, line_index: usize) {
    let newline = region.lines[line_index].newline.clone();
    if !newline.is_empty() {
        parser.emit_token(SyntaxKind::Newline, newline.end);
    }
}

/// joined 범위를 라인 조각 단위로 방출한다. 내용 조각은 `kind`,
/// 라인 전환('\n' 자리)은 개행 토큰으로, 라인 시작에서는 prefix를 방출한다.
/// 범위가 '\n' 자리에서 시작하거나 끝나는 경우도 정확히 처리한다.
pub(crate) fn emit_joined_range_as(
    parser: &mut Parser<'_>,
    region: &Region,
    joined_range: Range<usize>,
    kind: SyntaxKind,
) {
    let mut position = joined_range.start;
    while position < joined_range.end {
        let (line_index, column) = region.locate(position);
        let line_length = region.lines[line_index].content.len();
        if column == line_length {
            // '\n' 자리
            emit_line_newline(parser, region, line_index);
            position += 1;
            continue;
        }
        if column == 0 {
            emit_line_prefix(parser, region, line_index);
        }
        let piece_end_column = line_length.min(column + (joined_range.end - position));
        parser.emit_token(
            kind,
            region.lines[line_index].content.start + piece_end_column,
        );
        position += piece_end_column - column;
    }
}

/// 연료 소진 시의 안전 방출: 남은 라인들을 구조 없이 그대로 흘려보낸다.
pub(crate) fn emit_lines_flat(parser: &mut Parser<'_>, region: &Region, line_range: Range<usize>) {
    for index in line_range {
        emit_line_prefix(parser, region, index);
        parser.emit_token(SyntaxKind::Text, region.lines[index].content.end);
        emit_line_newline(parser, region, index);
    }
}
