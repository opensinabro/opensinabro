//! layout pass: 문서 전역 맥락 확정 (순수 함수).
//!
//! - 헤딩에 계층 번호("1.2")를 부여하고 목차를 수집해 `[목차]` 자리에 채운다.
//! - 각주에 번호를 부여하고(이름 각주는 미방출 구간 내 병합) 인라인을 참조로 치환한다.
//! - `[각주]` 자리에 그 시점까지 쌓인 각주를 채우고, 잔여 각주는 문서 끝에 방출한다.

use crate::resolve::Resolved;
use namumark_ast::TableAttributeScope;
use namumark_ir::{
    RenderBlock, RenderInline, RenderTable, RenderTableAttribute, RenderTree, RenderedFootnote,
    TableOfContentsEntry, TableStyleProperty, TextStyle,
};
use std::collections::HashMap;

pub(crate) fn layout(resolved: Resolved) -> RenderTree {
    let mut state = Layout {
        heading_stack: Vec::new(),
        table_of_contents: Vec::new(),
        pending_footnotes: Vec::new(),
        reference_count: 0,
    };
    let mut blocks = resolved.blocks;
    state.walk_blocks(&mut blocks);
    // 문서 끝까지 방출되지 않은 각주는 마지막에 제 문단으로 나온다.
    if !state.pending_footnotes.is_empty() {
        blocks.push(RenderBlock::Paragraph(vec![
            RenderInline::FootnoteSection {
                notes: std::mem::take(&mut state.pending_footnotes),
            },
        ]));
    }
    // 목차는 문서 전체 헤딩이 확정된 뒤에야 완성되므로 별도 패스로 채운다.
    fill_table_of_contents(&mut blocks, &state.table_of_contents);
    RenderTree {
        redirect: resolved.redirect,
        blocks,
        categories: resolved.categories,
    }
}

struct Layout {
    /// (헤딩 수준, 그 수준의 현재 번호)
    heading_stack: Vec<(u8, u32)>,
    table_of_contents: Vec<TableOfContentsEntry>,
    /// 아직 `[각주]`로 방출되지 않은 각주
    pending_footnotes: Vec<RenderedFootnote>,
    /// 지금까지 나온 각주 참조 수. 다음 참조의 번호가 되고, 무명 각주의 라벨이 된다.
    reference_count: u32,
}

impl Layout {
    fn walk_blocks(&mut self, blocks: &mut Vec<RenderBlock>) {
        for block in blocks {
            match block {
                RenderBlock::Heading {
                    level,
                    number,
                    anchor,
                    content,
                    ..
                } => {
                    *number = self.next_heading_number(*level);
                    self.walk_inlines(content);
                    *anchor = plain_text_of(content);
                    self.table_of_contents.push(TableOfContentsEntry {
                        number: number.clone(),
                        depth: self.heading_stack.len() as u8,
                        title: table_of_contents_title(content),
                    });
                }
                RenderBlock::Paragraph(inlines) => self.walk_inlines(inlines),
                RenderBlock::Quote(blocks) | RenderBlock::Indent(blocks) => {
                    self.walk_blocks(blocks)
                }
                RenderBlock::List { items, .. } => {
                    for item in items {
                        self.walk_blocks(&mut item.blocks);
                    }
                }
                RenderBlock::Table(table) => {
                    propagate_column_attributes(table);
                    if let Some(caption) = &mut table.caption {
                        self.walk_inlines(caption);
                    }
                    for row in &mut table.rows {
                        for cell in &mut row.cells {
                            self.walk_blocks(&mut cell.blocks);
                        }
                    }
                }
                RenderBlock::HorizontalRule => {}
            }
        }
    }

    fn next_heading_number(&mut self, level: u8) -> String {
        while self
            .heading_stack
            .last()
            .is_some_and(|(stack_level, _)| *stack_level > level)
        {
            self.heading_stack.pop();
        }
        match self.heading_stack.last_mut() {
            Some((stack_level, count)) if *stack_level == level => *count += 1,
            _ => self.heading_stack.push((level, 1)),
        }
        self.heading_stack
            .iter()
            .map(|(_, count)| count.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }

    fn walk_inlines(&mut self, inlines: &mut Vec<RenderInline>) {
        for inline in inlines {
            match inline {
                RenderInline::Footnote { name, content } => {
                    let name = name.take();
                    let mut content = std::mem::take(content);
                    self.walk_inlines(&mut content);
                    let (label, number, tooltip) = self.assign_footnote(name, content);
                    *inline = RenderInline::FootnoteReference {
                        label,
                        number,
                        tooltip,
                    };
                }
                RenderInline::Styled { content, .. }
                | RenderInline::Colored { content, .. }
                | RenderInline::Sized { content, .. } => self.walk_inlines(content),
                RenderInline::WikiStyle { blocks, .. } | RenderInline::Blocks(blocks) => {
                    self.walk_blocks(blocks)
                }
                RenderInline::FootnoteSection { notes } => {
                    *notes = std::mem::take(&mut self.pending_footnotes);
                }
                // 접기 문구는 글자라 각주도 목차도 들어 있지 않다.
                RenderInline::Folding { blocks, .. } => self.walk_blocks(blocks),
                RenderInline::DocumentLink { display, .. } => self.walk_inlines(display),
                RenderInline::ExternalLink {
                    display: Some(display),
                    ..
                } => self.walk_inlines(display),
                _ => {}
            }
        }
    }

    // 이름 각주는 미방출 구간 안에서 병합한다. 내용이 있는 첫 정의가 내용을 제공한다.
    /// 각주에 라벨·참조 번호를 주고 툴팁 글자를 함께 돌려준다.
    fn assign_footnote(
        &mut self,
        name: Option<String>,
        content: Vec<RenderInline>,
    ) -> (String, u32, String) {
        self.reference_count += 1;
        let number = self.reference_count;
        if let Some(name) = &name
            && let Some(footnote) = self
                .pending_footnotes
                .iter_mut()
                .find(|footnote| footnote.label == *name)
        {
            footnote.reference_numbers.push(number);
            if footnote.content.is_empty() {
                footnote.content = content;
            }
            return (
                footnote.label.clone(),
                number,
                plain_text_of(&footnote.content),
            );
        }
        let label = name.unwrap_or_else(|| number.to_string());
        let tooltip = plain_text_of(&content);
        self.pending_footnotes.push(RenderedFootnote {
            label: label.clone(),
            reference_numbers: vec![number],
            content,
        });
        (label, number, tooltip)
    }
}

/// 목차에 싣는 제목. 링크는 살리되 `[anchor()]`는 뺀다 — id가 겹치면 안 되기
/// 때문이다(렌더확정: the seed의 목차에는 `<a id='html'>`이 없다). 취소선은 목차에서
/// 벗겨 속내용만 남긴다(렌더확정: `==== --[[오리위키]]-- ====`가 본문 헤딩에서는 `<del>`이지만
/// 목차에서는 `<a>오리위키</a>`뿐이다). 다른 장식은 the seed가 목차에 그대로 두므로 건드리지 않는다.
fn table_of_contents_title(content: &[RenderInline]) -> Vec<RenderInline> {
    let mut title = Vec::new();
    for inline in content {
        match inline {
            RenderInline::Anchor { .. } => {}
            RenderInline::Styled {
                style: TextStyle::Strikethrough,
                content,
            } => title.extend(table_of_contents_title(content)),
            other => title.push(other.clone()),
        }
    }
    title
}

fn fill_table_of_contents(blocks: &mut Vec<RenderBlock>, entries: &[TableOfContentsEntry]) {
    for block in blocks {
        match block {
            RenderBlock::Paragraph(inlines) => fill_table_of_contents_inlines(inlines, entries),
            RenderBlock::Heading { content, .. } => {
                fill_table_of_contents_inlines(content, entries)
            }
            RenderBlock::Quote(blocks) | RenderBlock::Indent(blocks) => {
                fill_table_of_contents(blocks, entries)
            }
            RenderBlock::List { items, .. } => {
                for item in items {
                    fill_table_of_contents(&mut item.blocks, entries);
                }
            }
            RenderBlock::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        fill_table_of_contents(&mut cell.blocks, entries);
                    }
                }
            }
            _ => {}
        }
    }
}

fn fill_table_of_contents_inlines(inlines: &mut [RenderInline], entries: &[TableOfContentsEntry]) {
    for inline in inlines {
        match inline {
            RenderInline::TableOfContents {
                entries: inline_entries,
            } => *inline_entries = entries.to_vec(),
            RenderInline::WikiStyle { blocks, .. } | RenderInline::Blocks(blocks) => {
                fill_table_of_contents(blocks, entries)
            }
            RenderInline::Folding { blocks, .. } => fill_table_of_contents(blocks, entries),
            _ => {}
        }
    }
}

/// 인라인에서 글자만 뽑는다. 나무위키 목차가 제목의 서식·링크·앵커를 버리고
/// 글자만 싣기 때문에 필요하다(`== [[/TeX|수식]] ==` → `수식`).
fn plain_text_of(inlines: &[RenderInline]) -> String {
    let mut text = String::new();
    write_plain_text(inlines, &mut text);
    text
}

fn write_plain_text(inlines: &[RenderInline], output: &mut String) {
    for inline in inlines {
        match inline {
            RenderInline::Text(value) => output.push_str(value),
            RenderInline::Ruby { content, .. } => output.push_str(content),
            RenderInline::Styled { content, .. }
            | RenderInline::Colored { content, .. }
            | RenderInline::Sized { content, .. } => write_plain_text(content, output),
            RenderInline::DocumentLink { display, .. } => write_plain_text(display, output),
            RenderInline::ExternalLink { display, url, .. } => match display {
                Some(display) => write_plain_text(display, output),
                None => output.push_str(url),
            },
            // 줄바꿈은 글자가 아니다. 리터럴도 각주 툴팁에는 실리지 않는다
            // (렌더확정: `[* 앞 {{{[[ 파일:example.png]]}}}, … 형식]`의 툴팁이
            // the seed에서 `앞 ,  형식`이다 — 리터럴 자리가 통째로 빈다).
            RenderInline::LineBreak | RenderInline::Literal(_) => {}
            _ => {}
        }
    }
}

/// 열(`col`) 스코프 속성은 지정한 셀부터 **그 열의 아래 셀들**에 이어진다.
///
/// 나무위키의 `<colbgcolor=…>`가 열 전체를 칠하는 관용이 이것이다. 아래쪽 셀이 같은
/// 열에서 col 속성을 다시 지정하면 거기서부터 새 값으로 바뀐다.
/// 열 속성의 축(배경색·글자색·너비·…). 같은 축은 셀 지정이 열 상속을 덮고, 다른 축은
/// 독립적으로 상속된다.
fn attribute_kind(attribute: &RenderTableAttribute) -> std::mem::Discriminant<TableStyleProperty> {
    std::mem::discriminant(&attribute.property)
}

fn propagate_column_attributes(table: &mut RenderTable) {
    let mut inherited: HashMap<usize, Vec<RenderTableAttribute>> = HashMap::new();
    // 위 행의 `<|N>`이 아직 덮고 있는 열 → 남은 행 수. 그 열은 이 행의 셀이 쓰지 않는다.
    let mut occupied: HashMap<usize, u32> = HashMap::new();
    for row in &mut table.rows {
        let single_cell_row = row.cells.len() == 1;
        let mut column = 0usize;
        for cell in &mut row.cells {
            while occupied.contains_key(&column) {
                column += 1;
            }
            let declared: Vec<RenderTableAttribute> = cell
                .attributes
                .iter()
                .filter(|attribute| matches!(attribute.scope, TableAttributeScope::Column { .. }))
                .cloned()
                .collect();
            let span = cell.column_span.unwrap_or(1) as usize;
            // 합친 셀(`<-N>`)이 준 열 속성의 세로 전파 범위: 셀이 행 전체를 차지하거나
            // 스스로 `<width>`를 지정하면 시작 열에만, 그 외에는 걸치는 열 전체에 등록한다
            // (렌더확정: 나무위키 정보상자의 전폭 `<-2><colcolor=#fff>`와 심화 「활용 예시」의
            // 전폭 `<-4><colbgcolor=#fc6>`는 col 0에만, 알파위키 `<width=38%><-2><colcolor=#fff>`
            // 도 col 0에만 세로 전파되지만, 문법 도움말 「기본 헥스 코드」표의 폭 지정 없는
            // `<-3><colbgcolor=#f5f5f5>`는 걸친 세 열 모두에 전파된다).
            let start_column_only = single_cell_row
                || cell
                    .attributes
                    .iter()
                    .any(|attribute| matches!(attribute.property, TableStyleProperty::Width(_)));
            // 열 속성은 축(배경색·글자색·…)별로 독립 전파된다. 셀이 어떤 축을 스스로 지정해도
            // 지정하지 않은 축은 위 행에서 상속한다(렌더확정: `<colcolor=#fff>`가 걸린 열의
            // `<colbgcolor=#00a495>` 셀이 the seed에서 `color:#fff`도 함께 갖는다).
            let declared_kinds: Vec<_> = declared.iter().map(attribute_kind).collect();
            // 덮는 열 중 **오른쪽부터** 찾아 지정이 있는 열의 것을 따라간다(문법 도움말:
            // "입력한 숫자 값만큼 왼쪽/위의 셀의 것을 따라갑니다").
            let inherited_here: Vec<RenderTableAttribute> = (column..column + span)
                .rev()
                .find_map(|covered| inherited.get(&covered))
                .cloned()
                .unwrap_or_default();
            for attribute in inherited_here {
                if !declared_kinds.contains(&attribute_kind(&attribute)) {
                    cell.attributes.push(attribute);
                }
            }
            // 지정한 속성을 등록한다(제 축만 교체, 다른 축의 상속은 남긴다).
            let register_extent = if start_column_only { 1 } else { span };
            for attribute in declared {
                let kind = attribute_kind(&attribute);
                for covered in column..column + register_extent {
                    let entry = inherited.entry(covered).or_default();
                    entry.retain(|existing| attribute_kind(existing) != kind);
                    entry.push(attribute.clone());
                }
            }
            if let Some(rows) = cell.row_span.filter(|rows| *rows > 1) {
                for covered in column..column + span {
                    occupied.insert(covered, rows);
                }
            }
            column += span;
        }
        occupied.retain(|_, rows| {
            *rows -= 1;
            *rows > 0
        });
    }
}
