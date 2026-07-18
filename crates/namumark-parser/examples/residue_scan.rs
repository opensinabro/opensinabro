//! 나무마크 원문 뭉치를 파싱해 일반 텍스트에 남은 마크업(residue)을 집계한다.
//!
//! residue는 아직 지원하지 않는 문법을 가리키는 지표다. 실제 나무위키 문서를 모아
//! 이 도구에 통과시키면 미지원 문법이 잔여 줄로 드러난다.
//!
//! ```text
//! cargo run -p namumark-parser --example residue_scan -- 문서디렉토리/*.namu
//! ```
//!
//! 잔여 줄 중에는 원문 자체가 마커를 닫지 않은 오작성도 섞인다. 실제 화면과 대조해
//! 판별하는 방법은 docs/design/04-namuwiki-parity.md 참고.

use namumark_ast::{Block, Document, Inline};
use std::collections::BTreeMap;

const SUSPICIOUS_MARKERS: [&str; 8] = ["[[", "]]", "{{{", "}}}", "'''", "||", "[*", "#!"];

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("사용법: residue_scan <파일.namu>...");
        std::process::exit(2);
    }

    let mut marker_totals: BTreeMap<&str, usize> = BTreeMap::new();
    let mut macro_totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut documents_with_residue = 0;

    for path in &paths {
        let Ok(source) = std::fs::read_to_string(path) else {
            eprintln!("{path}: 읽기 실패");
            continue;
        };
        let document = namumark_parser::parse(&source);

        let mut plain_text = String::new();
        for block in &document.blocks() {
            collect_block_text(block, &mut plain_text);
        }
        collect_macro_names(&document, &mut macro_totals);

        let counts: Vec<(&str, usize)> = SUSPICIOUS_MARKERS
            .iter()
            .map(|marker| (*marker, plain_text.matches(marker).count()))
            .filter(|(_, count)| *count > 0)
            .collect();
        if counts.is_empty() {
            continue;
        }
        documents_with_residue += 1;
        println!("{path} ({} bytes)", source.len());
        for (marker, count) in counts {
            *marker_totals.entry(marker).or_default() += count;
            println!("  residue {marker} = {count}");
        }
        for line in plain_text.lines() {
            if SUSPICIOUS_MARKERS
                .iter()
                .any(|marker| line.contains(marker))
            {
                println!("  잔여: {line:?}");
            }
        }
    }

    println!(
        "\n== 요약: 문서 {}건 중 잔여 {documents_with_residue}건",
        paths.len()
    );
    for (marker, count) in &marker_totals {
        println!("  {marker} = {count}");
    }
    println!("== 매크로 사용 빈도");
    for (name, count) in &macro_totals {
        println!("  {name} = {count}");
    }
}

fn collect_macro_names(document: &Document, totals: &mut BTreeMap<String, usize>) {
    fn walk_inlines(inlines: &[Inline], totals: &mut BTreeMap<String, usize>) {
        for inline in inlines {
            match inline {
                Inline::Macro(macro_call) => {
                    *totals.entry(macro_call.name()).or_default() += 1
                }
                Inline::Bold(styled) => walk_inlines(&styled.content(), totals),
                Inline::Italic(styled) => walk_inlines(&styled.content(), totals),
                Inline::Strikethrough(styled) => walk_inlines(&styled.content(), totals),
                Inline::Underline(styled) => walk_inlines(&styled.content(), totals),
                Inline::Superscript(styled) => walk_inlines(&styled.content(), totals),
                Inline::Subscript(styled) => walk_inlines(&styled.content(), totals),
                Inline::Colored(colored) => walk_inlines(&colored.content(), totals),
                Inline::Sized(sized) => walk_inlines(&sized.content(), totals),
                Inline::Footnote(footnote) => walk_inlines(&footnote.content(), totals),
                Inline::Link(link) => {
                    if let Some(display) = &link.display() {
                        walk_inlines(display, totals)
                    }
                }
                _ => {}
            }
        }
    }

    fn walk_block(block: &Block, totals: &mut BTreeMap<String, usize>) {
        match block {
            Block::Heading(heading) => walk_inlines(&heading.content(), totals),
            Block::Paragraph(paragraph) => walk_inlines(&paragraph.inlines(), totals),
            Block::Quote(quote) => quote
                .blocks()
                .iter()
                .for_each(|block| walk_block(block, totals)),
            Block::Indent(indent) => indent
                .blocks()
                .iter()
                .for_each(|block| walk_block(block, totals)),
            Block::List(list) => list.items().iter().for_each(|item| {
                item.blocks()
                    .iter()
                    .for_each(|block| walk_block(block, totals))
            }),
            Block::Table(table) => table.rows().iter().for_each(|row| {
                row.cells.iter().for_each(|cell| {
                    cell.blocks
                        .iter()
                        .for_each(|block| walk_block(block, totals))
                })
            }),
            _ => {}
        }
    }

    document
        .blocks()
        .iter()
        .for_each(|block| walk_block(block, totals));
}

fn collect_block_text(block: &Block, output: &mut String) {
    match block {
        Block::Heading(heading) => collect_inline_text(&heading.content(), output),
        Block::Paragraph(paragraph) => collect_inline_text(&paragraph.inlines(), output),
        Block::Quote(quote) => {
            for block in &quote.blocks() {
                collect_block_text(block, output);
            }
        }
        Block::Indent(indent) => {
            for block in &indent.blocks() {
                collect_block_text(block, output);
            }
        }
        Block::List(list) => {
            for item in &list.items() {
                for block in &item.blocks() {
                    collect_block_text(block, output);
                }
            }
        }
        Block::Table(table) => {
            if let Some(caption) = &table.caption() {
                collect_inline_text(caption, output);
            }
            for row in &table.rows() {
                for cell in &row.cells {
                    for block in &cell.blocks {
                        collect_block_text(block, output);
                    }
                }
            }
        }
        // CodeBlock/Html/Comment/Redirect는 원문 그대로 보존되는 것이 정상이다.
        Block::Comment(_) | Block::Redirect(_) | Block::HorizontalRule => {}
    }
    output.push('\n');
}

fn collect_inline_text(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => {
                output.push_str(text);
                output.push('\n');
            }
            Inline::Bold(styled) => collect_inline_text(&styled.content(), output),
            Inline::Italic(styled) => collect_inline_text(&styled.content(), output),
            Inline::Strikethrough(styled) => collect_inline_text(&styled.content(), output),
            Inline::Underline(styled) => collect_inline_text(&styled.content(), output),
            Inline::Superscript(styled) => collect_inline_text(&styled.content(), output),
            Inline::Subscript(styled) => collect_inline_text(&styled.content(), output),
            Inline::Link(link) => {
                if let Some(display) = &link.display() {
                    collect_inline_text(display, output);
                }
            }
            Inline::Image(_) | Inline::Category(_) => {}
            Inline::Footnote(footnote) => collect_inline_text(&footnote.content(), output),
            Inline::Colored(colored) => collect_inline_text(&colored.content(), output),
            Inline::Sized(sized) => collect_inline_text(&sized.content(), output),
            Inline::WikiStyle(wiki_style) => {
                for block in &wiki_style.blocks() {
                    collect_block_text(block, output);
                }
            }
            Inline::Folding(folding) => {
                output.push_str(&folding.summary().to_string());
                for block in &folding.blocks() {
                    collect_block_text(block, output);
                }
            }
            Inline::Conditional(conditional) => {
                for block in &conditional.blocks() {
                    collect_block_text(block, output);
                }
            }
            Inline::LineBreak
            | Inline::Literal(_)
            | Inline::Macro(_)
            | Inline::Variable(_)
            | Inline::CodeBlock(_)
            | Inline::Html(_) => {}
        }
    }
}
