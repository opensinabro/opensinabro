//! 실제 나무위키 문서(fixtures/*.namu)를 파싱한 결과를 골든 스냅샷과 비교한다.
//!
//! 갱신: `UPDATE_GOLDEN=1 cargo test --test golden_tests`
//!
//! 골든 파일 헤더의 `residue` 줄은 파싱 후에도 일반 텍스트에 남은 마크업 토큰 수로,
//! 아직 지원하지 않는 문법을 가리키는 지표다. 0에 가까울수록 실제 화면과 가깝다.

use namumark::{Block, Document, Inline};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

#[test]
fn golden_fixtures() {
    let fixtures_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let golden_directory = fixtures_directory.join("golden");
    fs::create_dir_all(&golden_directory).expect("golden 디렉토리 생성 실패");
    let update = std::env::var("UPDATE_GOLDEN").is_ok();

    let mut slugs: Vec<String> = fs::read_dir(&fixtures_directory)
        .expect("fixtures 디렉토리 읽기 실패")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".namu").map(str::to_string)
        })
        .collect();
    slugs.sort();
    assert!(!slugs.is_empty(), "픽스처가 없습니다");

    let mut failures = Vec::new();
    for slug in &slugs {
        let source = fs::read_to_string(fixtures_directory.join(format!("{slug}.namu")))
            .expect("픽스처 읽기 실패");
        let document = namumark::parse(&source);
        let rendered = golden_content(slug, &source, &document);
        let golden_path = golden_directory.join(format!("{slug}.ast"));
        if update {
            fs::write(&golden_path, &rendered).expect("골든 파일 쓰기 실패");
            continue;
        }
        match fs::read_to_string(&golden_path) {
            Ok(expected) if expected == rendered => {}
            Ok(expected) => failures.push(format!(
                "{slug}: 골든 불일치 (UPDATE_GOLDEN=1 cargo test --test golden_tests 로 갱신)\n{}",
                first_difference(&expected, &rendered)
            )),
            Err(_) => failures.push(format!("{slug}: 골든 파일 없음 (UPDATE_GOLDEN=1 로 생성)")),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn first_difference(expected: &str, actual: &str) -> String {
    for (line_number, (expected_line, actual_line)) in
        expected.lines().zip(actual.lines()).enumerate()
    {
        if expected_line != actual_line {
            return format!(
                "  줄 {}:\n  - {}\n  + {}",
                line_number + 1,
                expected_line,
                actual_line
            );
        }
    }
    "  (길이만 다름)".to_string()
}

fn golden_content(slug: &str, source: &str, document: &Document) -> String {
    let mut output = String::new();
    writeln!(output, "# {slug}.namu — {} bytes", source.len()).unwrap();
    writeln!(output, "# residue: {}", residue_report(document)).unwrap();
    for block in &document.blocks {
        render_block(block, 0, &mut output);
    }
    output
}

// 파싱 후 일반 텍스트에 남아서는 안 되는 마크업 토큰을 집계한다.
fn residue_report(document: &Document) -> String {
    const SUSPICIOUS_MARKERS: [&str; 8] = ["[[", "]]", "{{{", "}}}", "'''", "||", "[*", "#!"];
    let mut plain_text = String::new();
    for block in &document.blocks {
        collect_block_text(block, &mut plain_text);
    }
    let counts: Vec<String> = SUSPICIOUS_MARKERS
        .iter()
        .map(|marker| format!("{marker}={}", plain_text.matches(marker).count()))
        .collect();
    counts.join(" ")
}

fn collect_block_text(block: &Block, output: &mut String) {
    match block {
        Block::Heading(heading) => collect_inline_text(&heading.content, output),
        Block::Paragraph(inlines) => collect_inline_text(inlines, output),
        Block::Quote(blocks) | Block::Indent(blocks) => {
            for block in blocks {
                collect_block_text(block, output);
            }
        }
        Block::List(list) => {
            for item in &list.items {
                for block in &item.blocks {
                    collect_block_text(block, output);
                }
            }
        }
        Block::Table(table) => {
            if let Some(caption) = &table.caption {
                collect_inline_text(caption, output);
            }
            for row in &table.rows {
                for cell in &row.cells {
                    for block in &cell.blocks {
                        collect_block_text(block, output);
                    }
                }
            }
        }
        Block::WikiStyle(wiki_style) => {
            for block in &wiki_style.blocks {
                collect_block_text(block, output);
            }
        }
        Block::Folding(folding) => {
            collect_inline_text(&folding.summary, output);
            for block in &folding.blocks {
                collect_block_text(block, output);
            }
        }
        Block::Colored(colored) => {
            for block in &colored.blocks {
                collect_block_text(block, output);
            }
        }
        Block::Sized(sized) => {
            for block in &sized.blocks {
                collect_block_text(block, output);
            }
        }
        // CodeBlock/Html/Comment/Redirect는 원문 그대로 보존되는 것이 정상이다.
        Block::CodeBlock(_) | Block::Html(_) | Block::Comment(_) | Block::Redirect(_) => {}
        Block::HorizontalRule => {}
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
            Inline::Bold(content)
            | Inline::Italic(content)
            | Inline::Strikethrough(content)
            | Inline::Underline(content)
            | Inline::Superscript(content)
            | Inline::Subscript(content) => collect_inline_text(content, output),
            Inline::Link(link) => {
                if let Some(display) = &link.display {
                    collect_inline_text(display, output);
                }
            }
            Inline::Image(_) | Inline::Category(_) => {}
            Inline::Footnote(footnote) => collect_inline_text(&footnote.content, output),
            Inline::Colored(colored) => collect_inline_text(&colored.content, output),
            Inline::Sized(sized) => collect_inline_text(&sized.content, output),
            Inline::LineBreak | Inline::Literal(_) | Inline::Macro(_) | Inline::Html(_) => {}
        }
    }
}

fn render_block(block: &Block, depth: usize, output: &mut String) {
    push_indent(output, depth);
    match block {
        Block::Heading(heading) => {
            writeln!(
                output,
                "Heading level={} folded={}: {}",
                heading.level,
                heading.folded,
                render_inlines(&heading.content)
            )
            .unwrap();
        }
        Block::Paragraph(inlines) => {
            writeln!(output, "Paragraph: {}", render_inlines(inlines)).unwrap();
        }
        Block::HorizontalRule => {
            output.push_str("HorizontalRule\n");
        }
        Block::Quote(blocks) => {
            output.push_str("Quote:\n");
            render_children(blocks, depth + 1, output);
        }
        Block::Indent(blocks) => {
            output.push_str("Indent:\n");
            render_children(blocks, depth + 1, output);
        }
        Block::List(list) => {
            writeln!(output, "List kind={:?}:", list.kind).unwrap();
            for item in &list.items {
                push_indent(output, depth + 1);
                match item.start_number {
                    Some(start_number) => writeln!(output, "Item start={start_number}:").unwrap(),
                    None => output.push_str("Item:\n"),
                }
                render_children(&item.blocks, depth + 2, output);
            }
        }
        Block::Table(table) => {
            match &table.caption {
                Some(caption) => {
                    writeln!(output, "Table caption={}:", render_inlines(caption)).unwrap()
                }
                None => output.push_str("Table:\n"),
            }
            for row in &table.rows {
                push_indent(output, depth + 1);
                output.push_str("Row:\n");
                for cell in &row.cells {
                    push_indent(output, depth + 2);
                    let mut header = format!(
                        "Cell span={}x{} align={:?}",
                        cell.column_span, cell.row_span, cell.horizontal_alignment
                    );
                    if let Some(vertical_alignment) = cell.vertical_alignment {
                        write!(header, " vertical={vertical_alignment:?}").unwrap();
                    }
                    if !cell.attributes.is_empty() {
                        let attributes: Vec<String> = cell
                            .attributes
                            .iter()
                            .map(|attribute| {
                                format!(
                                    "{:?}/{}{}",
                                    attribute.scope,
                                    attribute.name,
                                    attribute
                                        .value
                                        .as_ref()
                                        .map(|value| format!("={value}"))
                                        .unwrap_or_default()
                                )
                            })
                            .collect();
                        write!(header, " attrs=[{}]", attributes.join(" ")).unwrap();
                    }
                    writeln!(output, "{header}:").unwrap();
                    render_children(&cell.blocks, depth + 3, output);
                }
            }
        }
        Block::CodeBlock(code_block) => {
            writeln!(
                output,
                "CodeBlock language={:?}: {:?}",
                code_block.language, code_block.source
            )
            .unwrap();
        }
        Block::WikiStyle(wiki_style) => {
            writeln!(
                output,
                "WikiStyle style={:?} dark_style={:?}:",
                wiki_style.style, wiki_style.dark_style
            )
            .unwrap();
            render_children(&wiki_style.blocks, depth + 1, output);
        }
        Block::Folding(folding) => {
            writeln!(
                output,
                "Folding summary={}:",
                render_inlines(&folding.summary)
            )
            .unwrap();
            render_children(&folding.blocks, depth + 1, output);
        }
        Block::Colored(colored) => {
            let dark = colored
                .dark_color
                .as_ref()
                .map(|dark_color| format!(",{dark_color}"))
                .unwrap_or_default();
            writeln!(output, "ColoredBlock color={}{dark}:", colored.color).unwrap();
            render_children(&colored.blocks, depth + 1, output);
        }
        Block::Sized(sized) => {
            writeln!(output, "SizedBlock level={:+}:", sized.level).unwrap();
            render_children(&sized.blocks, depth + 1, output);
        }
        Block::Html(html) => {
            writeln!(output, "Html: {html:?}").unwrap();
        }
        Block::Comment(comment) => {
            writeln!(output, "Comment: {comment:?}").unwrap();
        }
        Block::Redirect(target) => {
            writeln!(output, "Redirect: {target:?}").unwrap();
        }
    }
}

fn render_children(blocks: &[Block], depth: usize, output: &mut String) {
    for block in blocks {
        render_block(block, depth, output);
    }
}

fn push_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn render_inlines(inlines: &[Inline]) -> String {
    let rendered: Vec<String> = inlines.iter().map(render_inline).collect();
    rendered.join(" ")
}

fn render_inline(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) => format!("{text:?}"),
        Inline::LineBreak => "<br>".to_string(),
        Inline::Bold(content) => format!("Bold({})", render_inlines(content)),
        Inline::Italic(content) => format!("Italic({})", render_inlines(content)),
        Inline::Strikethrough(content) => format!("Strike({})", render_inlines(content)),
        Inline::Underline(content) => format!("Underline({})", render_inlines(content)),
        Inline::Superscript(content) => format!("Sup({})", render_inlines(content)),
        Inline::Subscript(content) => format!("Sub({})", render_inlines(content)),
        Inline::Literal(text) => format!("Literal({text:?})"),
        Inline::Link(link) => {
            let anchor = link
                .anchor
                .as_ref()
                .map(|anchor| format!(" #{anchor}"))
                .unwrap_or_default();
            match &link.display {
                Some(display) => format!(
                    "Link({:?}{anchor} | {})",
                    link.target,
                    render_inlines(display)
                ),
                None => format!("Link({:?}{anchor})", link.target),
            }
        }
        Inline::Image(image) => {
            let options: Vec<String> = image
                .options
                .iter()
                .map(|option| match &option.value {
                    Some(value) => format!("{}={value}", option.name),
                    None => option.name.clone(),
                })
                .collect();
            if options.is_empty() {
                format!("Image({:?})", image.file_name)
            } else {
                format!("Image({:?} {})", image.file_name, options.join("&"))
            }
        }
        Inline::Category(category) => format!("Category({:?})", category.name),
        Inline::Footnote(footnote) => {
            let name = footnote
                .name
                .as_ref()
                .map(|name| format!("{name} "))
                .unwrap_or_default();
            format!("Footnote({name}| {})", render_inlines(&footnote.content))
        }
        Inline::Macro(macro_call) => match &macro_call.argument {
            Some(argument) => format!("Macro({} {argument:?})", macro_call.name),
            None => format!("Macro({})", macro_call.name),
        },
        Inline::Colored(colored) => {
            let dark = colored
                .dark_color
                .as_ref()
                .map(|dark_color| format!(",{dark_color}"))
                .unwrap_or_default();
            format!(
                "Colored({}{dark} | {})",
                colored.color,
                render_inlines(&colored.content)
            )
        }
        Inline::Sized(sized) => {
            format!(
                "Sized({:+} | {})",
                sized.level,
                render_inlines(&sized.content)
            )
        }
        Inline::Html(html) => format!("InlineHtml({html:?})"),
    }
}
