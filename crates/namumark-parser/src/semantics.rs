//! 표기 판정(namumark-text)의 자체 어휘를 의미 모델(namumark-ast) 타입으로 매핑한다.

use namumark_ast::{
    HorizontalAlignment, ImageOption, ListKind, TableAttribute, TableAttributeScope,
    VerticalAlignment,
};
use namumark_text::{
    CellAlignment, CellOption, CellOptionScope, CellShape, ListMarkerKind, VerticalPosition,
};

pub(crate) fn list_kind(kind: ListMarkerKind) -> ListKind {
    match kind {
        ListMarkerKind::Unordered => ListKind::Unordered,
        ListMarkerKind::Decimal => ListKind::Decimal,
        ListMarkerKind::LowerAlphabet => ListKind::LowerAlphabet,
        ListMarkerKind::UpperAlphabet => ListKind::UpperAlphabet,
        ListMarkerKind::LowerRoman => ListKind::LowerRoman,
        ListMarkerKind::UpperRoman => ListKind::UpperRoman,
    }
}

pub(crate) struct CellSemantics {
    pub column_span_override: Option<u32>,
    pub row_span: Option<u32>,
    pub horizontal_alignment: Option<HorizontalAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub attributes: Vec<TableAttribute>,
}

pub(crate) fn cell_semantics(shape: &CellShape<'_>) -> CellSemantics {
    let mut semantics = CellSemantics {
        column_span_override: None,
        row_span: None,
        horizontal_alignment: None,
        vertical_alignment: None,
        attributes: Vec::new(),
    };
    for option in &shape.options {
        match option {
            CellOption::Alignment(alignment) => {
                semantics.horizontal_alignment = Some(horizontal_alignment(*alignment));
            }
            CellOption::ColumnSpan(span) => semantics.column_span_override = Some(*span),
            CellOption::RowSpan {
                span,
                vertical_position,
            } => {
                semantics.row_span = Some(*span);
                if let Some(vertical_position) = vertical_position {
                    semantics.vertical_alignment = Some(match vertical_position {
                        VerticalPosition::Top => VerticalAlignment::Top,
                        VerticalPosition::Bottom => VerticalAlignment::Bottom,
                    });
                }
            }
            CellOption::Flag { scope, name } => semantics.attributes.push(TableAttribute {
                scope: attribute_scope(*scope, semantics.column_span_override),
                name: (*name).to_string(),
                value: None,
            }),
            CellOption::Attribute { scope, name, value } => {
                semantics.attributes.push(TableAttribute {
                    scope: attribute_scope(*scope, semantics.column_span_override),
                    name: (*name).to_string(),
                    value: Some(crate::lower::template_of(value)),
                });
            }
            CellOption::BackgroundColor(value) => semantics.attributes.push(TableAttribute {
                scope: TableAttributeScope::Cell,
                name: "bgcolor".to_string(),
                value: Some(crate::lower::template_of(value)),
            }),
        }
    }
    semantics
}

fn horizontal_alignment(alignment: CellAlignment) -> HorizontalAlignment {
    match alignment {
        CellAlignment::Left => HorizontalAlignment::Left,
        CellAlignment::Center => HorizontalAlignment::Center,
        CellAlignment::Right => HorizontalAlignment::Right,
    }
}

fn attribute_scope(scope: CellOptionScope, columns: Option<u32>) -> TableAttributeScope {
    match scope {
        CellOptionScope::Cell => TableAttributeScope::Cell,
        CellOptionScope::Row => TableAttributeScope::Row,
        // 여기까지 아는 칸 수만큼의 열에 걸린다 — 나무위키는 옵션을 왼쪽부터 처리한다.
        CellOptionScope::Column => TableAttributeScope::Column {
            columns: columns.unwrap_or(1),
        },
        CellOptionScope::Table => TableAttributeScope::Table,
    }
}

pub(crate) fn image_options(source: &str) -> Vec<ImageOption> {
    source
        .split('&')
        .filter(|part| !part.trim().is_empty())
        .map(|part| match part.split_once('=') {
            Some((name, value)) => ImageOption {
                name: name.trim().to_string(),
                value: Some(crate::lower::template_of(value.trim())),
            },
            None => ImageOption {
                name: part.trim().to_string(),
                value: None,
            },
        })
        .collect()
}
