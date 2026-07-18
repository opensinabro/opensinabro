//! 우리가 해석하지 못한 문법을 문서 뭉치에서 빈도순으로 집계한다.
//!
//! residue 스캔(`namumark-parser`의 `residue_scan` 예제)이 "텍스트로 흘린 마크업"을 잡는다면,
//! 이쪽은 그 반대편의 사각지대를 본다 — 마커는 정상 소비했지만 우리가 **조용히 버린** 것들이다.
//! 미지원 매크로, 무시한 이미지 옵션, 옵션 파싱이 중단된 셀 토큰이 여기 잡힌다.
//! residue는 0인데 화면은 다른 부류라 렌더 대조 없이는 놓치기 쉽다.

use namumark_ast::{Block, Document, Inline};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// resolve pass가 실제로 해석하는 매크로. 여기 없으면 원문이 그대로 노출된다.
const KNOWN_MACROS: [&str; 17] = [
    "br",
    "clearfix",
    "anchor",
    "math",
    "date",
    "datetime",
    "age",
    "dday",
    "youtube",
    "kakaotv",
    "nicovideo",
    "ruby",
    "include",
    "목차",
    "tableofcontents",
    "각주",
    "footnote",
];

/// resolve pass가 실제로 해석하는 이미지 옵션.
const KNOWN_IMAGE_OPTIONS: [&str; 5] = ["width", "height", "align", "bgcolor", "theme"];

#[derive(Default)]
struct Findings {
    macros: BTreeMap<String, usize>,
    image_options: BTreeMap<String, usize>,
}

pub fn run(paths: &[PathBuf]) {
    let mut findings = Findings::default();
    let mut scanned = 0;
    for path in paths {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        scanned += 1;
        let document = namumark_parser::parse(&source);
        walk_document(&document, &mut findings);
    }

    println!("== 문서 {scanned}건 스캔");
    report_section(
        "우리가 해석하지 못하는 매크로 (원문이 화면에 그대로 노출됨)",
        &findings.macros,
    );
    report_section(
        "우리가 무시하는 이미지 옵션 (조용히 폐기됨 — residue로는 안 잡힘)",
        &findings.image_options,
    );
    println!(
        "\n빈도가 높은 항목은 the seed가 실제로 지원할 가능성이 큽니다.\n\
         실제 동작 확인은 `parity compare` 또는 알파위키 렌더 대조로 하세요."
    );
}

fn report_section(title: &str, counts: &BTreeMap<String, usize>) {
    println!("\n-- {title}");
    if counts.is_empty() {
        println!("  (없음)");
        return;
    }
    let mut ranked: Vec<(&String, &usize)> = counts.iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
    for (name, count) in ranked {
        println!("  {count:>5}  {name}");
    }
}

fn walk_document(document: &Document, findings: &mut Findings) {
    walk_blocks(&document.blocks(), findings);
}

fn walk_blocks(blocks: &[Block], findings: &mut Findings) {
    for block in blocks {
        walk_block(block, findings);
    }
}

fn walk_block(block: &Block, findings: &mut Findings) {
    match block {
        Block::Heading(heading) => walk_inlines(&heading.content(), findings),
        Block::Paragraph(paragraph) => walk_inlines(&paragraph.inlines(), findings),
        Block::Quote(quote) => walk_blocks(&quote.blocks(), findings),
        Block::Indent(indent) => walk_blocks(&indent.blocks(), findings),
        Block::List(list) => list
            .items()
            .iter()
            .for_each(|item| walk_blocks(&item.blocks(), findings)),
        Block::Table(table) => {
            if let Some(caption) = &table.caption() {
                walk_inlines(caption, findings);
            }
            table.rows().iter().for_each(|row| {
                row.cells
                    .iter()
                    .for_each(|cell| walk_blocks(&cell.blocks, findings))
            })
        }
        Block::Comment(_) | Block::Redirect(_) | Block::HorizontalRule => {}
    }
}

fn walk_inlines(inlines: &[Inline], findings: &mut Findings) {
    for inline in inlines {
        match inline {
            Inline::Macro(macro_call) => {
                let raw = macro_call.name();
                let name = raw.to_ascii_lowercase();
                if !KNOWN_MACROS.contains(&name.as_str()) {
                    *findings.macros.entry(raw).or_default() += 1;
                }
            }
            Inline::Image(image) => {
                for option in &image.options() {
                    let name = option.name.to_ascii_lowercase();
                    if !KNOWN_IMAGE_OPTIONS.contains(&name.as_str()) {
                        *findings
                            .image_options
                            .entry(option.name.clone())
                            .or_default() += 1;
                    }
                }
            }
            Inline::Bold(styled) => walk_inlines(&styled.content(), findings),
            Inline::Italic(styled) => walk_inlines(&styled.content(), findings),
            Inline::Strikethrough(styled) => walk_inlines(&styled.content(), findings),
            Inline::Underline(styled) => walk_inlines(&styled.content(), findings),
            Inline::Superscript(styled) => walk_inlines(&styled.content(), findings),
            Inline::Subscript(styled) => walk_inlines(&styled.content(), findings),
            Inline::Colored(colored) => walk_inlines(&colored.content(), findings),
            Inline::Sized(sized) => walk_inlines(&sized.content(), findings),
            Inline::WikiStyle(wiki_style) => walk_blocks(&wiki_style.blocks(), findings),
            // 접기 문구는 글자라 매크로도 옵션도 들어 있지 않다.
            Inline::Folding(folding) => walk_blocks(&folding.blocks(), findings),
            Inline::Conditional(conditional) => walk_blocks(&conditional.blocks(), findings),
            Inline::CodeBlock(_) => {}
            Inline::Footnote(footnote) => walk_inlines(&footnote.content(), findings),
            Inline::Link(link) => {
                if let Some(display) = &link.display() {
                    walk_inlines(display, findings)
                }
            }
            Inline::Text(_)
            | Inline::LineBreak
            | Inline::Literal(_)
            | Inline::Variable(_)
            | Inline::Html(_) => {}
            Inline::Category(_) => {}
        }
    }
}
