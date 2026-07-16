//! 무손실 구문 트리 → 의미 모델(Document) 변환.
//!
//! 트리는 구조만 담고 있으므로 leaf 의미(색상 값, 앵커, 셀 옵션 등)는
//! 토큰 텍스트에 기존 검증 로직(crate::text)을 적용해 계산한다.

use crate::semantics;
use namumark_ast::{
    Block, Category, CodeBlock, ColoredBlock, ColoredText, Document, Folding, Footnote, Heading,
    HorizontalAlignment, Image, Inline, Link, List, ListItem, ListKind, Macro, SizedBlock,
    SizedText, Table, TableCell, TableRow, WikiStyle,
};
use namumark_syntax::{NodeOrToken, SyntaxKind, SyntaxNode};
use namumark_text as text;

pub(crate) fn lower_document(root: &SyntaxNode) -> Document {
    Document {
        blocks: lower_block_children(root),
    }
}

fn lower_block_children(node: &SyntaxNode) -> Vec<Block> {
    node.children()
        .filter_map(|child| lower_block(&child))
        .collect()
}

fn lower_block(node: &SyntaxNode) -> Option<Block> {
    Some(match node.kind() {
        SyntaxKind::Paragraph => Block::Paragraph(assemble_inlines(node)),
        SyntaxKind::Heading => {
            let marker = first_marker_text(node).unwrap_or_default();
            Block::Heading(Heading {
                level: marker.bytes().filter(|&byte| byte == b'=').count() as u8,
                folded: marker.contains('#'),
                content: assemble_inlines(node),
            })
        }
        SyntaxKind::HorizontalRule => Block::HorizontalRule,
        SyntaxKind::Quote => Block::Quote(lower_block_children(node)),
        SyntaxKind::Indent => Block::Indent(lower_block_children(node)),
        SyntaxKind::List => lower_list(node),
        SyntaxKind::Table => lower_table(node),
        SyntaxKind::CodeBlock => lower_code_block(node),
        SyntaxKind::HtmlBlock => Block::Html(raw_content_text(node)),
        SyntaxKind::WikiStyle => {
            let header = first_marker_text(node).unwrap_or_default();
            let rest = text::strip_directive(header.trim_start_matches("{{{"), "#!wiki")
                .unwrap_or_default();
            let (style, dark_style, _) = text::parse_wiki_style_attributes(rest);
            Block::WikiStyle(WikiStyle {
                style,
                dark_style,
                blocks: lower_block_children(node),
            })
        }
        SyntaxKind::Folding => {
            let summary = node
                .children()
                .find(|child| child.kind() == SyntaxKind::FoldingSummary)
                .map(|summary| assemble_inlines(&summary))
                .unwrap_or_default();
            Block::Folding(Folding {
                summary,
                blocks: lower_block_children(node),
            })
        }
        SyntaxKind::ColoredBlock => {
            let header = first_marker_text(node).unwrap_or_default();
            let (color, dark_color, _) =
                text::parse_color_marker(header.trim_start_matches("{{{"))?;
            Block::Colored(ColoredBlock {
                color,
                dark_color,
                blocks: lower_block_children(node),
            })
        }
        SyntaxKind::SizedBlock => {
            let header = first_marker_text(node).unwrap_or_default();
            let (level, _) = text::parse_size_marker(header.trim_start_matches("{{{"))?;
            Block::Sized(SizedBlock {
                level,
                blocks: lower_block_children(node),
            })
        }
        SyntaxKind::Comment => {
            let line = raw_text_tokens(node);
            Block::Comment(line.strip_prefix("##").unwrap_or(&line).to_string())
        }
        SyntaxKind::Redirect => {
            let line = raw_text_tokens(node);
            Block::Redirect(text::parse_redirect(&line)?)
        }
        _ => return None,
    })
}

fn lower_list(node: &SyntaxNode) -> Block {
    let mut kind = ListKind::Unordered;
    let mut items = Vec::new();
    for (index, item) in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::ListItem)
        .enumerate()
    {
        let marker_text = item
            .children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .find(|token| token.kind() == SyntaxKind::ListMarker)
            .map(|token| token.text().to_string())
            .unwrap_or_default();
        let (item_kind, start_number) = match text::list_marker(&marker_text) {
            Some(marker) => (semantics::list_kind(marker.kind), marker.start_number),
            None => (ListKind::Unordered, None),
        };
        if index == 0 {
            kind = item_kind;
        }
        items.push(ListItem {
            start_number,
            blocks: lower_block_children(&item),
        });
    }
    Block::List(List { kind, items })
}

fn lower_table(node: &SyntaxNode) -> Block {
    let mut caption = None;
    let mut rows = Vec::new();
    for row_node in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::TableRow)
    {
        let mut cells = Vec::new();
        // 다음 셀의 자동 colspan = 직전 파이프 런의 쌍 수. 캡션은 가상 `||` 한 쌍을 더한다.
        let mut pending_pairs = 0usize;
        for element in row_node.children_with_tokens() {
            match element {
                NodeOrToken::Token(token) => {
                    if token.kind() == SyntaxKind::Marker {
                        let text = token.text();
                        if !text.is_empty() && text.bytes().all(|byte| byte == b'|') {
                            pending_pairs += text.len() / 2;
                        }
                    }
                }
                NodeOrToken::Node(child) => match child.kind() {
                    SyntaxKind::TableCaption => {
                        caption = Some(assemble_inlines(&child));
                        pending_pairs += 1;
                    }
                    SyntaxKind::TableCell => {
                        cells.push(lower_table_cell(&child, pending_pairs));
                        pending_pairs = 0;
                    }
                    _ => {}
                },
            }
        }
        if !cells.is_empty() {
            rows.push(TableRow { cells });
        }
    }
    Block::Table(Table { caption, rows })
}

fn lower_table_cell(node: &SyntaxNode, pending_pairs: usize) -> TableCell {
    // 마커 배치 규칙(문법 계층과의 계약): `<...>` 나열은 옵션, 콘텐츠 앞의 " " 마커는
    // 선행 정렬 공백, 콘텐츠 뒤의 " " 마커는 후행 정렬 공백이다.
    let mut options_text = String::new();
    let mut leading_space = false;
    let mut trailing_space = false;
    let mut seen_content = false;
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) if token.kind() == SyntaxKind::Marker => {
                let token_text = token.text();
                if token_text.starts_with('<') && !seen_content {
                    options_text.push_str(token_text);
                } else if !token_text.trim().is_empty() {
                    continue;
                } else if seen_content {
                    trailing_space = true;
                } else {
                    leading_space = true;
                }
            }
            NodeOrToken::Node(_) => seen_content = true,
            _ => {}
        }
    }

    let shape = text::cell_shape(&options_text);
    let cell = semantics::cell_semantics(&shape);
    let horizontal_alignment = cell.horizontal_alignment.unwrap_or({
        if leading_space && trailing_space {
            HorizontalAlignment::Center
        } else if leading_space {
            HorizontalAlignment::Right
        } else {
            HorizontalAlignment::Left
        }
    });
    TableCell {
        column_span: cell
            .column_span_override
            .unwrap_or(pending_pairs as u32)
            .max(1),
        row_span: cell.row_span,
        horizontal_alignment,
        vertical_alignment: cell.vertical_alignment,
        attributes: cell.attributes,
        blocks: lower_block_children(node),
    }
}

fn lower_code_block(node: &SyntaxNode) -> Block {
    let language = node
        .children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(|token| token.kind() == SyntaxKind::Marker)
        .find_map(|token| {
            let text = token.text().trim_start_matches("{{{").to_string();
            let rest = text::strip_directive(&text, "#!syntax")?.trim().to_string();
            Some(rest)
        })
        .filter(|language| !language.is_empty());
    Block::CodeBlock(CodeBlock {
        language,
        source: raw_content_text(node),
    })
}

/// CodeBlock/HtmlBlock의 원문 복원: Text 토큰과 개행만 모으고 가장자리 개행 하나씩 제거.
fn raw_content_text(node: &SyntaxNode) -> String {
    let mut content = String::new();
    for element in node.children_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            match token.kind() {
                SyntaxKind::Text => content.push_str(token.text()),
                SyntaxKind::Newline => content.push('\n'),
                _ => {}
            }
        }
    }
    let content = content.strip_prefix('\n').unwrap_or(&content);
    let content = content.strip_suffix('\n').unwrap_or(content);
    content.to_string()
}

/// Text 토큰만 이어붙인다 (Comment/Redirect처럼 한 줄짜리 원문 복원용).
fn raw_text_tokens(node: &SyntaxNode) -> String {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(|token| token.kind() == SyntaxKind::Text)
        .map(|token| token.text().to_string())
        .collect()
}

fn first_marker_text(node: &SyntaxNode) -> Option<String> {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .find(|token| token.kind() == SyntaxKind::Marker)
        .map(|token| token.text().to_string())
}

// ---- 인라인 ----

fn assemble_inlines(node: &SyntaxNode) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut buffer = String::new();
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) => match token.kind() {
                SyntaxKind::Text => buffer.push_str(token.text()),
                SyntaxKind::Escaped => buffer.push_str(&token.text()[1..]),
                SyntaxKind::Newline => {
                    flush_text(&mut buffer, &mut inlines);
                    inlines.push(Inline::LineBreak);
                }
                _ => {}
            },
            NodeOrToken::Node(child) => {
                if let Some(inline) = lower_inline(&child) {
                    flush_text(&mut buffer, &mut inlines);
                    inlines.push(inline);
                }
            }
        }
    }
    flush_text(&mut buffer, &mut inlines);
    inlines
}

fn flush_text(buffer: &mut String, inlines: &mut Vec<Inline>) {
    if !buffer.is_empty() {
        inlines.push(Inline::Text(std::mem::take(buffer)));
    }
}

fn lower_inline(node: &SyntaxNode) -> Option<Inline> {
    Some(match node.kind() {
        SyntaxKind::Bold => Inline::Bold(assemble_inlines(node)),
        SyntaxKind::Italic => Inline::Italic(assemble_inlines(node)),
        SyntaxKind::Strikethrough => Inline::Strikethrough(assemble_inlines(node)),
        SyntaxKind::Underline => Inline::Underline(assemble_inlines(node)),
        SyntaxKind::Superscript => Inline::Superscript(assemble_inlines(node)),
        SyntaxKind::Subscript => Inline::Subscript(assemble_inlines(node)),
        SyntaxKind::Literal => Inline::Literal(raw_text_tokens(node)),
        SyntaxKind::InlineHtml => Inline::Html(raw_text_tokens(node)),
        SyntaxKind::ColoredText => {
            let marker = first_marker_text(node)?;
            let (color, dark_color, _) = text::parse_color_marker(&marker[3..])?;
            Inline::Colored(ColoredText {
                color,
                dark_color,
                content: assemble_inlines(node),
            })
        }
        SyntaxKind::SizedText => {
            let marker = first_marker_text(node)?;
            let (level, _) = text::parse_size_marker(&marker[3..])?;
            Inline::Sized(SizedText {
                level,
                content: assemble_inlines(node),
            })
        }
        SyntaxKind::Link => {
            let node_text = node.text().to_string();
            let body = &node_text[2..node_text.len() - 2];
            let (target, has_display) = match body.split_once('|') {
                Some((target, _)) => (target, true),
                None => (body, false),
            };
            let target = match target.strip_prefix(':') {
                Some(stripped)
                    if text::strip_link_prefix(
                        stripped,
                        &["파일:", "file:", "분류:", "category:"],
                    )
                    .is_some() =>
                {
                    stripped
                }
                _ => target,
            };
            let (target, anchor) = text::split_anchor(target);
            Inline::Link(Link {
                target: target.to_string(),
                anchor,
                display: has_display.then(|| assemble_inlines(node)),
            })
        }
        SyntaxKind::Image => {
            let node_text = node.text().to_string();
            let body = &node_text[2..node_text.len() - 2];
            let (target, display) = match body.split_once('|') {
                Some((target, display)) => (target, display),
                None => (body, ""),
            };
            let file_name = text::strip_link_prefix(target, &["파일:", "file:"])?;
            Inline::Image(Image {
                file_name: file_name.to_string(),
                options: semantics::image_options(display),
            })
        }
        SyntaxKind::Category => {
            let node_text = node.text().to_string();
            let body = &node_text[2..node_text.len() - 2];
            let target = body
                .split_once('|')
                .map(|(target, _)| target)
                .unwrap_or(body);
            let name = text::strip_link_prefix(target, &["분류:", "category:"])?;
            Inline::Category(Category {
                name: name.to_string(),
            })
        }
        SyntaxKind::Footnote => {
            let node_text = node.text().to_string();
            let body = &node_text[2..node_text.len() - 1];
            let name = match body.split_once(' ') {
                Some((name, _)) => name,
                None => body,
            };
            Inline::Footnote(Footnote {
                name: (!name.is_empty()).then(|| name.to_string()),
                content: assemble_inlines(node),
            })
        }
        SyntaxKind::MacroCall => {
            let node_text = node.text().to_string();
            let body = &node_text[1..node_text.len() - 1];
            let (name, argument) = match body.split_once('(') {
                Some((name, argument)) => (
                    name,
                    Some(argument.strip_suffix(')').unwrap_or(argument).to_string()),
                ),
                None => (body, None),
            };
            Inline::Macro(Macro {
                name: name.to_string(),
                argument,
            })
        }
        _ => return None,
    })
}
