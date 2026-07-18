//! 무손실 구문 트리 → 의미 모델(Document) 변환.
//!
//! 트리는 구조만 담고 있으므로 leaf 의미(색상 값, 앵커, 셀 옵션 등)는
//! 토큰 텍스트에 기존 검증 로직(crate::text)을 적용해 계산한다.

use crate::semantics;
use namumark_ast::{
    Block, Category, CodeBlock, ColoredText, Conditional, Document, Folding, Footnote, Fragment,
    Heading, HorizontalAlignment, Image, Inline, Link, List, ListItem, ListKind, Macro, SizedText,
    Table, TableCell, TableRow, Template, Variable, WikiStyle,
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
            let marker = first_token_text(node, SyntaxKind::DelimiterOpen).unwrap_or_default();
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
        SyntaxKind::Comment => {
            let line = raw_text_tokens(node);
            Block::Comment(line.strip_prefix("##").unwrap_or(&line).to_string())
        }
        SyntaxKind::Redirect => {
            // 지시자(`#redirect `)는 별도 토큰이라 Text에는 대상만 남는다.
            Block::Redirect(template_of(raw_text_tokens(node).trim()))
        }
        _ => return None,
    })
}

/// `{{{#색상}}}`·`{{{+N}}}`이 여러 줄에 걸친 경우의 내용.
/// 이 그룹들은 서식일 뿐이라 안쪽 블록을 인라인으로 편다.
fn block_children_as_inlines(node: &SyntaxNode) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for block in lower_block_children(node) {
        match block {
            Block::Paragraph(mut content) => {
                if !inlines.is_empty() {
                    inlines.push(Inline::LineBreak);
                }
                inlines.append(&mut content);
            }
            // 서식 그룹 안의 표·리스트는 인라인으로 펼 수 없다. 드문 형태라 버린다.
            _ => continue,
        }
    }
    inlines
}

fn lower_list(node: &SyntaxNode) -> Block {
    let mut kind = ListKind::Unordered;
    let mut items = Vec::new();
    for (index, item) in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::ListItem)
        .enumerate()
    {
        // 여러 줄 항목은 마커를 하위 영역의 줄머리로 옮기므로 자손까지 본다.
        // 문서 순서상 처음 나오는 것이 이 항목의 마커다(중첩 리스트의 것보다 앞선다).
        // 종류(`1.`) 바로 뒤에 시작번호(`#42`)가 오면 이어 붙여 `1.#42`로 되짚는다.
        let mut tokens = item
            .descendants_with_tokens()
            .filter_map(NodeOrToken::into_token);
        let marker_text = match tokens.find(|token| token.kind() == SyntaxKind::ListMarker) {
            Some(bullet) => {
                let mut text = bullet.text().to_string();
                if let Some(number) = tokens.next()
                    && number.kind() == SyntaxKind::ListStartNumber
                {
                    text.push_str(number.text());
                }
                text
            }
            None => String::new(),
        };
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
                    if token.kind() == SyntaxKind::Separator {
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
        // 셀이 없는 행도 행이다 — 위 행의 rowspan에 덮인 자리를 이렇게 비워 둔다
        // (렌더확정: 원문 `||||` 행이 the seed에서 `<tr class='wiki-table-tr'></tr>`다).
        rows.push(TableRow { cells });
    }
    Block::Table(Table { caption, rows })
}

fn lower_table_cell(node: &SyntaxNode, pending_pairs: usize) -> TableCell {
    // 마커 배치 규칙(문법 계층과의 계약): `<...>` 나열은 옵션, 콘텐츠 앞의 " " 마커는
    // 선행 정렬 공백, 콘텐츠 뒤의 " " 마커는 후행 정렬 공백이다.
    // 마커 배치 규칙(문법 계층과의 계약): 내용 노드 앞의 옵션 토큰(`<…>`)은 옵션이고,
    // AlignmentSpace 토큰은 내용 앞뒤의 정렬 결정 공백이다.
    let mut options_text = String::new();
    let mut leading_space = false;
    let mut trailing_space = false;
    let mut seen_content = false;
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) => match token.kind() {
                SyntaxKind::AlignmentSpace => {
                    if seen_content {
                        trailing_space = true;
                    } else {
                        leading_space = true;
                    }
                }
                _ if !seen_content => options_text.push_str(token.text()),
                _ => {}
            },
            NodeOrToken::Node(_) => seen_content = true,
        }
    }

    let shape = text::cell_shape(&options_text);
    let cell = semantics::cell_semantics(&shape);
    // 나무위키는 정렬을 **지정한** 셀에만 text-align을 방출한다. 공백 없는 셀은
    // 기본(왼쪽)이라 지정이 없는 것으로 남긴다.
    let horizontal_alignment = cell.horizontal_alignment.or({
        if leading_space && trailing_space {
            Some(HorizontalAlignment::Center)
        } else if leading_space {
            Some(HorizontalAlignment::Right)
        } else if trailing_space {
            Some(HorizontalAlignment::Left)
        } else {
            None
        }
    });
    TableCell {
        // 지정한 대로만 싣는다 — the seed는 `<-1>`로 적힌 1도 `colspan='1'`로 낸다.
        column_span: cell
            .column_span_override
            .or_else(|| (pending_pairs > 1).then_some(pending_pairs as u32)),
        row_span: cell.row_span,
        horizontal_alignment,
        vertical_alignment: cell.vertical_alignment,
        attributes: cell.attributes,
        blocks: lower_block_children(node),
    }
}

fn lower_code_block(node: &SyntaxNode) -> CodeBlock {
    let language =
        first_token_text(node, SyntaxKind::CodeLanguage).filter(|language| !language.is_empty());
    CodeBlock {
        language,
        source: raw_content_text(node),
    }
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

/// `#!wiki` 헤더의 스타일 속성부를 되짚는다. 지시자 토큰 다음부터 첫 내용 노드 전까지의
/// 토큰(속성 이름·`=`·값·공백)을 이어붙이면 원래 속성 문자열이 된다.
fn wiki_attributes_text(node: &SyntaxNode) -> String {
    let mut text = String::new();
    let mut after_directive = false;
    for element in node.children_with_tokens() {
        match element {
            NodeOrToken::Token(token) => {
                if token.kind() == SyntaxKind::Directive {
                    after_directive = true;
                } else if after_directive {
                    text.push_str(token.text());
                }
            }
            NodeOrToken::Node(_) => break,
        }
    }
    text
}

fn first_token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    node.children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .find(|token| token.kind() == kind)
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
        SyntaxKind::InlineHtml => Inline::Html(template_of(&raw_text_tokens(node))),
        SyntaxKind::ColoredText => {
            let value = first_token_text(node, SyntaxKind::ColorValue)?;
            let (color, dark_color) = text::parse_color_specification(&value)?;
            Inline::Colored(ColoredText {
                color,
                dark_color,
                content: assemble_inlines(node),
            })
        }
        SyntaxKind::SizedText => {
            let value = first_token_text(node, SyntaxKind::SizeLevel)?;
            let (level, _) = text::parse_size_marker(&value)?;
            Inline::Sized(SizedText {
                level,
                content: assemble_inlines(node),
            })
        }
        SyntaxKind::Link => {
            let node_text = node.text().to_string();
            let body = &node_text[2..node_text.len() - 2];
            let (target, display) = text::split_link_body(body);
            let has_display = display.is_some();
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
                target: template_of(&text::unescape(target)),
                anchor: anchor
                    .as_deref()
                    .map(|anchor| template_of(&text::unescape(anchor))),
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
            // `파일:` 접두사 뒤 공백은 파일 이름이 아니다(렌더확정: `[[파일: 특별행정구기.svg]]`가
            // the seed에서 이미지로 뜬다 — 공백째 이름으로 삼으면 파일을 못 찾는다).
            let file_name = text::strip_link_prefix(target, &["파일:", "file:"])?.trim();
            Inline::Image(Image {
                file_name: template_of(file_name),
                options: semantics::image_options(display),
            })
        }
        SyntaxKind::WikiStyle => {
            let attributes = wiki_attributes_text(node);
            let (style, dark_style, _) = text::parse_wiki_style_attributes(&attributes);
            Inline::WikiStyle(WikiStyle {
                style: style.as_deref().map(template_of),
                dark_style: dark_style.as_deref().map(template_of),
                blocks: lower_block_children(node),
            })
        }
        SyntaxKind::Folding => Inline::Folding(Folding {
            summary: template_of(
                &node
                    .children()
                    .find(|child| child.kind() == SyntaxKind::FoldingSummary)
                    .map(|summary| summary.text().to_string())
                    .unwrap_or_default(),
            ),
            blocks: lower_block_children(node),
        }),
        SyntaxKind::Conditional => Inline::Conditional(Conditional {
            expression: node
                .children()
                .find(|child| child.kind() == SyntaxKind::ConditionExpression)
                .map(|expression| expression.text().to_string())
                .unwrap_or_default(),
            blocks: lower_block_children(node),
        }),
        SyntaxKind::CodeBlock => Inline::CodeBlock(lower_code_block(node)),
        SyntaxKind::HtmlBlock => Inline::Html(template_of(&raw_content_text(node))),
        SyntaxKind::ColoredBlock => {
            let value = first_token_text(node, SyntaxKind::ColorValue)?;
            let (color, dark_color) = text::parse_color_specification(&value)?;
            Inline::Colored(ColoredText {
                color,
                dark_color,
                content: block_children_as_inlines(node),
            })
        }
        SyntaxKind::SizedBlock => {
            let value = first_token_text(node, SyntaxKind::SizeLevel)?;
            let (level, _) = text::parse_size_marker(&value)?;
            Inline::Sized(SizedText {
                level,
                content: block_children_as_inlines(node),
            })
        }
        SyntaxKind::TemplateVariable => {
            let shape = text::variable_shape(&node.text().to_string())?;
            let node_text = node.text().to_string();
            Inline::Variable(Variable {
                name: node_text[shape.name.clone()].to_string(),
                default: shape
                    .default
                    .clone()
                    .map(|range| node_text[range].to_string()),
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
                argument: argument.as_deref().map(template_of),
            })
        }
        _ => return None,
    })
}

/// 토큰 텍스트에서 틀 인자 표기를 갈라 `Template`을 만든다.
///
/// 인라인 문맥의 인자는 구문 트리가 이미 노드로 끊어 주지만, 헤더나 옵션처럼
/// 마커 토큰 하나로 들어오는 문자열은 여기서 갈라낸다(leaf 의미 계산).
pub(crate) fn template_of(source: &str) -> Template {
    let mut fragments = Vec::new();
    let mut pending = String::new();
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(shape) = text::variable_shape(rest) {
            if !pending.is_empty() {
                fragments.push(Fragment::Text(std::mem::take(&mut pending)));
            }
            fragments.push(Fragment::Variable(Variable {
                name: rest[shape.name.clone()].to_string(),
                default: shape.default.clone().map(|range| rest[range].to_string()),
            }));
            rest = &rest[shape.length..];
            continue;
        }
        let next = rest
            .char_indices()
            .skip(1)
            .find(|(_, character)| *character == '@')
            .map(|(index, _)| index)
            .unwrap_or(rest.len());
        pending.push_str(&rest[..next]);
        rest = &rest[next..];
    }
    if !pending.is_empty() {
        fragments.push(Fragment::Text(pending));
    }
    Template(fragments)
}
