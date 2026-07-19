import type { CSSProperties, ReactNode } from "react";
import type { RenderTable } from "@/lib/namumark/RenderTable";
import type { RenderTableAttribute } from "@/lib/namumark/RenderTableAttribute";
import type { RenderTableCell } from "@/lib/namumark/RenderTableCell";
import type { RenderTableRow } from "@/lib/namumark/RenderTableRow";
import type { TableStyleProperty } from "@/lib/namumark/TableStyleProperty";
import { Blocks } from "./blocks";
import type { RenderContext } from "./context";
import { Inlines } from "./inlines";
import { classNames } from "./style";

/**
 * 표 스코프 속성은 두 군데로 나뉜다. 정렬은 감싸는 div의 클래스로, 너비는 그 div의
 * style로 가고(이때 표 자신은 100%가 된다), 색·테두리는 표 자신의 style로 간다.
 */
export function Table({
  table,
  context,
}: {
  table: RenderTable;
  context: RenderContext;
}): ReactNode {
  const attributes = tableScopedAttributes(table);
  const width = attributes.findLast(
    (property) => property.type === "width",
  )?.width;
  const alignment = attributes.findLast(
    (property) => property.type === "align",
  )?.alignment;

  return (
    <div
      className={classNames(
        "wiki-table-wrap",
        alignment !== undefined && `table-${alignment}`,
      )}
      style={width !== undefined ? { width } : undefined}
    >
      <table className="wiki-table" style={tableStyle(attributes)}>
        {table.caption !== null && (
          <caption>
            <Inlines inlines={table.caption} context={context} />
          </caption>
        )}
        <tbody>
          {table.rows.map((row, index) => (
            <Row key={index} row={row} context={context} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Row({
  row,
  context,
}: {
  row: RenderTableRow;
  context: RenderContext;
}): ReactNode {
  const style: CSSProperties = {};
  for (const cell of row.cells) {
    for (const attribute of cell.attributes) {
      if (attribute.scope.type === "row") {
        applyCellStyle(style, attribute.property);
      }
    }
  }
  return (
    <tr className="wiki-table-tr" style={hasAny(style) ? style : undefined}>
      {row.cells.map((cell, index) => (
        <Cell key={index} cell={cell} context={context} />
      ))}
    </tr>
  );
}

function Cell({
  cell,
  context,
}: {
  cell: RenderTableCell;
  context: RenderContext;
}): ReactNode {
  // `<nopad>`는 style이 아니라 클래스로 나간다.
  const noPadding = cell.attributes.some(
    (attribute) =>
      attribute.scope.type === "cell" &&
      attribute.property.type === "noPadding",
  );

  const style: CSSProperties = {};
  if (cell.horizontalAlignment !== null) {
    style.textAlign = cell.horizontalAlignment;
  }
  if (cell.verticalAlignment !== null) {
    style.verticalAlign = cell.verticalAlignment;
  }
  // 같은 속성을 셀과 열이 함께 주면 셀이 이긴다
  // (bgcolor > colbgcolor > rowbgcolor > tablebgcolor).
  for (const attribute of cell.attributes) {
    const overriddenByCell =
      attribute.scope.type === "column" &&
      cell.attributes.some(
        (other) =>
          other.scope.type === "cell" &&
          other.property.type === attribute.property.type,
      );
    if (
      !overriddenByCell &&
      (attribute.scope.type === "cell" || attribute.scope.type === "column")
    ) {
      applyCellStyle(style, attribute.property);
    }
  }

  return (
    <td
      className={noPadding ? "wiki-table-nopadding" : undefined}
      colSpan={cell.columnSpan ?? undefined}
      rowSpan={cell.rowSpan ?? undefined}
      style={style}
    >
      {/* 빈 셀도 빈 문단 하나는 갖는다. */}
      {cell.blocks.length === 0 ? (
        <div className="wiki-paragraph" />
      ) : (
        <Blocks blocks={cell.blocks} context={context} />
      )}
    </td>
  );
}

/** 표의 모든 셀에 흩어져 있는 표 스코프 속성. 같은 속성이 겹치면 마지막이 이긴다. */
function tableScopedAttributes(table: RenderTable): TableStyleProperty[] {
  return table.rows
    .flatMap((row) => row.cells)
    .flatMap((cell: RenderTableCell) => cell.attributes)
    .filter((attribute: RenderTableAttribute) => attribute.scope.type === "table")
    .map((attribute) => attribute.property);
}

function tableStyle(properties: TableStyleProperty[]): CSSProperties | undefined {
  const style: CSSProperties = {};
  for (const property of properties) {
    switch (property.type) {
      // 너비가 지정되면 감싸는 div가 그 폭을 갖고 표는 그 안을 채운다.
      case "width":
        style.width = "100%";
        break;
      case "backgroundColor":
        style.backgroundColor = property.color;
        break;
      case "color":
        style.color = property.color;
        break;
      case "borderColor":
        style.border = `2px solid ${property.color}`;
        break;
      case "height":
        style.height = property.height;
        break;
      case "textAlign":
        style.textAlign = property.alignment;
        break;
      case "align":
      case "noPadding":
        break;
    }
  }
  return hasAny(style) ? style : undefined;
}

/** 행·열·셀의 style로 나가는 속성만 얹는다. 정렬·패딩은 여기서 아무것도 쓰지 않는다. */
function applyCellStyle(style: CSSProperties, property: TableStyleProperty) {
  switch (property.type) {
    case "backgroundColor":
      style.backgroundColor = property.color;
      break;
    case "color":
      style.color = property.color;
      break;
    case "width":
      style.width = property.width;
      break;
    case "height":
      style.height = property.height;
      break;
    case "textAlign":
      style.textAlign = property.alignment;
      break;
    default:
      break;
  }
}

function hasAny(style: CSSProperties): boolean {
  return Object.keys(style).length > 0;
}
