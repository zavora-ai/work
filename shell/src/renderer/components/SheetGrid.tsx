/**
 * The grid.
 *
 * Draws a [`Sheet`] exactly as the Core sent it. It formats nothing, evaluates
 * nothing, and parses nothing — every `display` string arrived ready to read. What
 * this component owns is interaction: which cell is selected, what the formula bar
 * shows, and which sheet is in front.
 *
 * Real row and column headers come from the sheet's own position, so a model that
 * starts at row 5 shows row 5 rather than renumbering from one.
 */

import { useState } from "react";

import { cellRef, columnName, type Cell, type GridModel, type Sheet } from "../../shared/grid.ts";
import { Field } from "./primitives.tsx";

const ROW_HEADER = 40;

export function SheetGrid({
  model,
  note,
  onEdit,
}: {
  model: GridModel;
  /** A short aside for the sheet strip, where there is room for it. */
  note?: string;
  /**
   * Change a cell yourself. Absent where the file cannot be written, in which case the bar
   * shows the value and does not pretend to accept one.
   */
  onEdit?: (sheet: string, cell: string, value: string) => void;
}) {
  const [active, setActive] = useState(model.active);
  // What the User has typed but not yet committed. Undefined means "showing the cell".
  const [typed, setTyped] = useState<string | undefined>(undefined);
  const [selected, setSelected] = useState<{ row: number; col: number }>({
    row: model.sheets[model.active]?.firstRow ?? 0,
    col: (model.sheets[model.active]?.firstCol ?? 0) + 3,
  });

  const sheet = model.sheets[active];
  if (!sheet) return null;

  const cell = cellAt(sheet, selected.row, selected.col);

  return (
    // Three bands: the formula bar, the grid, and the sheets. The grid is the only one
    // that scrolls, so the reference and the sheet names stay put however far down the
    // User is — which is the whole reason a spreadsheet puts them at the edges.
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        background: "#fff",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "6px 10px",
          borderBottom: "1px solid var(--border)",
          background: "var(--card)",
        }}
      >
        <span
          style={{
            fontSize: 11.5,
            fontWeight: 650,
            color: "var(--ink-soft)",
            minWidth: 30,
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
          }}
        >
          {cellRef(selected.row, selected.col)}
        </span>
        {onEdit ? (
          <Field
            // Typing here is the User's own edit. It goes the same way an agent's does.
            value={typed ?? cell?.formula ?? cell?.display ?? ""}
            mono
            label={cellRef(selected.row, selected.col)}
            onChange={(event) => setTyped(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                const value = typed ?? "";
                setTyped(undefined);
                onEdit(sheet.name, cellRef(selected.row, selected.col), value);
              } else if (event.key === "Escape") {
                setTyped(undefined);
              }
            }}
            style={{ flex: 1, padding: "6px 10px", fontSize: 12 }}
          />
        ) : (
          <Field
            value={cell?.formula ?? cell?.display ?? ""}
            mono
            style={{ flex: 1, padding: "6px 10px", fontSize: 12 }}
          />
        )}
      </div>

      <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
        <Grid
          sheet={sheet}
          selected={selected}
          onSelect={(at) => {
            setTyped(undefined);
            setSelected(at);
          }}
        />
      </div>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 4,
          padding: "0 10px",
          borderTop: "1px solid var(--border)",
          background: "var(--card)",
          minHeight: 30,
        }}
      >
        {model.sheets.length > 1 ? (
          <div style={{ display: "flex", gap: 4 }}>
          {model.sheets.map((candidate, index) => {
            const on = index === active;
            return (
              <button
                key={candidate.name}
                type="button"
                onClick={() => {
                  setActive(index);
                  setSelected({ row: candidate.firstRow, col: candidate.firstCol });
                }}
                aria-current={on ? "true" : undefined}
                style={{
                  border: "1px solid",
                  borderColor: on ? "var(--border-strong)" : "transparent",
                  // The strip sits under the grid, so the active sheet opens upward
                  // into it rather than downward.
                  borderTopColor: on ? "#fff" : "var(--border)",
                  background: on ? "#fff" : "transparent",
                  color: on ? "#26241f" : "var(--muted)",
                  fontWeight: on ? 650 : 500,
                  fontSize: 12,
                  padding: "5px 12px",
                  borderRadius: "0 0 6px 6px",
                  cursor: "pointer",
                }}
              >
                {candidate.name}
                </button>
              );
            })}
          </div>
        ) : null}
        {note ? (
          <span className="hint" style={{ marginLeft: "auto", fontSize: 11 }}>
            {note}
          </span>
        ) : null}
      </div>
    </div>
  );
}

function Grid({
  sheet,
  selected,
  onSelect,
}: {
  sheet: Sheet;
  selected: { row: number; col: number };
  onSelect: (at: { row: number; col: number }) => void;
}) {
  const cols = sheet.rows[0]?.length ?? 0;
  if (cols === 0) {
    return (
      <p className="hint" style={{ padding: 12 }}>
        This sheet is empty.
      </p>
    );
  }

  const template = `${ROW_HEADER}px repeat(${cols}, minmax(0, 1fr))`;

  return (
    <div
      role="table"
      style={{
        border: "1px solid var(--border)",
        borderRadius: 8,
        overflow: "hidden",
        background: "#fff",
        fontSize: 11.5,
      }}
    >
      <div
        role="row"
        style={{
          display: "grid",
          gridTemplateColumns: template,
          background: "#f7f6f3",
          borderBottom: "1px solid var(--border)",
          fontWeight: 650,
          color: "var(--ink-soft)",
          textAlign: "center",
        }}
      >
        <div style={{ padding: "5px 0" }} />
        {Array.from({ length: cols }, (_, index) => (
          <div key={index} role="columnheader" style={{ padding: "5px 0" }}>
            {columnName(sheet.firstCol + index)}
          </div>
        ))}
      </div>

      {sheet.rows.map((row, rowIndex) => {
        const absoluteRow = sheet.firstRow + rowIndex;
        const rowSelected = absoluteRow === selected.row;
        return (
          <div key={absoluteRow} role="row" style={{ display: "grid", gridTemplateColumns: template }}>
            <div
              role="rowheader"
              style={{
                padding: "6px 0",
                textAlign: "center",
                background: rowSelected ? "#eef0f4" : "#f7f6f3",
                color: rowSelected ? "var(--ink-soft)" : "var(--faint)",
                fontWeight: rowSelected ? 650 : 400,
              }}
            >
              {absoluteRow + 1}
            </div>
            {row.map((cell, colIndex) => {
              const absoluteCol = sheet.firstCol + colIndex;
              const isSelected = rowSelected && absoluteCol === selected.col;
              return (
                <CellView
                  key={absoluteCol}
                  cell={cell}
                  selected={isSelected}
                  label={cellRef(absoluteRow, absoluteCol)}
                  onSelect={() => onSelect({ row: absoluteRow, col: absoluteCol })}
                />
              );
            })}
          </div>
        );
      })}
    </div>
  );
}

function CellView({
  cell,
  selected,
  label,
  onSelect,
}: {
  cell: Cell;
  selected: boolean;
  label: string;
  onSelect: () => void;
}) {
  const style = cell.style ?? {};
  return (
    <div
      role="cell"
      tabIndex={0}
      aria-label={`${label} ${cell.display}`}
      onClick={onSelect}
      onFocus={onSelect}
      style={{
        padding: "6px 9px",
        textAlign: style.align ?? (cell.numeric ? "right" : "left"),
        fontWeight: style.bold ? 650 : 400,
        fontStyle: style.italic ? "italic" : "normal",
        color: cell.error ? "var(--warn-fg)" : (style.colour ?? "#3d3933"),
        background: style.background ?? "transparent",
        outline: selected ? "2px solid #6f8fa8" : "none",
        outlineOffset: -2,
        cursor: "cell",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
      }}
      title={cell.error ? `That cell holds an error: ${cell.error}` : undefined}
    >
      {cell.display}
    </div>
  );
}

function cellAt(sheet: Sheet, row: number, col: number): Cell | undefined {
  return sheet.rows[row - sheet.firstRow]?.[col - sheet.firstCol];
}
