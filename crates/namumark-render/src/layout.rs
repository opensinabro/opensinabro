//! layout pass: 문서 전역 맥락 확정 (순수 함수).
//!
//! - 헤딩에 계층 번호("1.2")를 부여하고 목차를 수집해 `[목차]` 자리에 채운다.
//! - 각주에 번호를 부여하고(이름 각주는 미방출 구간 내 병합) 인라인을 참조로 치환한다.
//! - `[각주]` 자리에 그 시점까지 쌓인 각주를 채우고, 잔여 각주는 문서 끝에 방출한다.

use crate::resolve::Resolved;
use namumark_ir::{RenderBlock, RenderInline, RenderTree, RenderedFootnote, TableOfContentsEntry};

pub(crate) fn layout(resolved: Resolved) -> RenderTree {
    let mut state = Layout {
        heading_stack: Vec::new(),
        table_of_contents: Vec::new(),
        pending_footnotes: Vec::new(),
        created_footnote_count: 0,
    };
    let mut blocks = resolved.blocks;
    state.walk_blocks(&mut blocks);
    if !state.pending_footnotes.is_empty() {
        blocks.push(RenderBlock::FootnoteSection {
            notes: std::mem::take(&mut state.pending_footnotes),
        });
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
    /// 지금까지 만든 각주 수. 무명 각주 라벨(전역 연속 번호)에 쓰인다.
    created_footnote_count: usize,
}

impl Layout {
    fn walk_blocks(&mut self, blocks: &mut Vec<RenderBlock>) {
        for block in blocks {
            match block {
                RenderBlock::Heading {
                    level,
                    number,
                    content,
                    ..
                } => {
                    *number = self.next_heading_number(*level);
                    self.walk_inlines(content);
                    self.table_of_contents.push(TableOfContentsEntry {
                        number: number.clone(),
                        depth: self.heading_stack.len() as u8,
                        title: content.clone(),
                    });
                }
                RenderBlock::Paragraph(inlines) => self.walk_inlines(inlines),
                RenderBlock::Quote(blocks)
                | RenderBlock::Indent(blocks)
                | RenderBlock::WikiStyle { blocks, .. }
                | RenderBlock::Colored { blocks, .. }
                | RenderBlock::Sized { blocks, .. } => self.walk_blocks(blocks),
                RenderBlock::Folding { summary, blocks } => {
                    self.walk_inlines(summary);
                    self.walk_blocks(blocks);
                }
                RenderBlock::List { items, .. } => {
                    for item in items {
                        self.walk_blocks(&mut item.blocks);
                    }
                }
                RenderBlock::Table(table) => {
                    if let Some(caption) = &mut table.caption {
                        self.walk_inlines(caption);
                    }
                    for row in &mut table.rows {
                        for cell in &mut row.cells {
                            self.walk_blocks(&mut cell.blocks);
                        }
                    }
                }
                RenderBlock::FootnoteSection { notes } => {
                    *notes = std::mem::take(&mut self.pending_footnotes);
                }
                RenderBlock::HorizontalRule
                | RenderBlock::CodeBlock { .. }
                | RenderBlock::Html(_)
                | RenderBlock::TableOfContents { .. } => {}
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
                    let (label, reference_index) = self.assign_footnote(name, content);
                    *inline = RenderInline::FootnoteReference {
                        label,
                        reference_index,
                    };
                }
                RenderInline::Styled { content, .. }
                | RenderInline::Colored { content, .. }
                | RenderInline::Sized { content, .. } => self.walk_inlines(content),
                RenderInline::DocumentLink {
                    display: Some(display),
                    ..
                }
                | RenderInline::ExternalLink {
                    display: Some(display),
                    ..
                } => self.walk_inlines(display),
                _ => {}
            }
        }
    }

    // 이름 각주는 미방출 구간 안에서 병합한다. 내용이 있는 첫 정의가 내용을 제공한다.
    fn assign_footnote(
        &mut self,
        name: Option<String>,
        content: Vec<RenderInline>,
    ) -> (String, usize) {
        if let Some(name) = &name
            && let Some(footnote) = self
                .pending_footnotes
                .iter_mut()
                .find(|footnote| footnote.label == *name)
        {
            let reference_index = footnote.reference_count;
            footnote.reference_count += 1;
            if footnote.content.is_empty() {
                footnote.content = content;
            }
            return (footnote.label.clone(), reference_index);
        }
        self.created_footnote_count += 1;
        let label = name.unwrap_or_else(|| self.created_footnote_count.to_string());
        self.pending_footnotes.push(RenderedFootnote {
            label: label.clone(),
            reference_count: 1,
            content,
        });
        (label, 0)
    }
}

fn fill_table_of_contents(blocks: &mut Vec<RenderBlock>, entries: &[TableOfContentsEntry]) {
    for block in blocks {
        match block {
            RenderBlock::TableOfContents {
                entries: block_entries,
            } => *block_entries = entries.to_vec(),
            RenderBlock::Quote(blocks)
            | RenderBlock::Indent(blocks)
            | RenderBlock::WikiStyle { blocks, .. }
            | RenderBlock::Colored { blocks, .. }
            | RenderBlock::Sized { blocks, .. }
            | RenderBlock::Folding { blocks, .. } => fill_table_of_contents(blocks, entries),
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
