/**
 * The grid.
 *
 * Draws a [`Sheet`] exactly as the Core sent it. It formats nothing, evaluates nothing, and
 * parses nothing — every `display` string arrived ready to read. What this component owns is
 * interaction, and that is most of what makes a grid feel like a spreadsheet rather than a
 * table: where the selection is, how the keyboard moves it, what a range is, and what copying
 * one gives you.
 *
 * Real row and column headers come from the sheet's own position, so a model that starts at row
 * 5 shows row 5 rather than renumbering from one. Headers stay put while the cells scroll,
 * because a spreadsheet whose headers scroll away has lost the thing headers are for.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { cellRef, columnName, type Cell, type GridModel, type Sheet } from "../../shared/grid.ts";
import { ChartView } from "./ChartView.tsx";
import { Field } from "./primitives.tsx";

/// Whether the User was working in the grid when it was last taken down.
///
/// Editing a cell reloads the file, which remounts this component and drops the focus with it —
/// so the next key after an edit went nowhere, and undo appeared not to work. Kept outside the
/// component because that is the only thing that survives the remount. Not focused unless the
/// grid had focus already: taking it while the User is typing in the chat box would be worse
/// than losing it.
let wasWorkingHere = false;

const ROW_HEADER = 44;
const DEFAULT_COLUMN = 96;
const ROW_HEIGHT = 25;

/** A rectangle of cells, in absolute coordinates. */
export interface Range {
  fromRow: number;
  fromCol: number;
  toRow: number;
  toCol: number;
}

function normalise(range: Range): Range {
  return {
    fromRow: Math.min(range.fromRow, range.toRow),
    toRow: Math.max(range.fromRow, range.toRow),
    fromCol: Math.min(range.fromCol, range.toCol),
    toCol: Math.max(range.fromCol, range.toCol),
  };
}

function within(range: Range, row: number, col: number): boolean {
  const box = normalise(range);
  return row >= box.fromRow && row <= box.toRow && col >= box.fromCol && col <= box.toCol;
}

export function SheetGrid({
  model,
  note,
  onEdit,
  onEditMany,
  onSelection,
  onUndo,
}: {
  model: GridModel;
  /** A short aside for the sheet strip, where there is room for it. */
  note?: string;
  /**
   * Change a cell yourself. Absent where the file cannot be written, in which case the bar
   * shows the value and does not pretend to accept one.
   */
  onEdit?: (sheet: string, cell: string, value: string) => void;
  /** Change several at once, as pasting does. Falls back to one at a time when absent. */
  onEditMany?: (sheet: string, cells: { cell: string; value: string }[]) => void;
  /**
   * What is selected now, so a toolbar outside the grid can act on it.
   *
   * Reported rather than kept private, because formatting belongs to the selection and the
   * controls for it live above the file, not inside the grid.
   */
  onSelection?: (at: {
    sheet: string;
    /** "B6" or "B6:D9". */
    range: string;
    /** The row the selection starts at, counting from 1 as the header shows it. */
    row: number;
    /** The column letter the selection starts at. */
    column: string;
  }) => void;
  /** Put the last change back. Absent where the file cannot be written. */
  onUndo?: () => void;
}) {
  const [active, setActive] = useState(model.active);
  // What the User has typed but not yet committed. Undefined means "showing the cell".
  const [typed, setTyped] = useState<string | undefined>(undefined);
  // Typing straight into a cell, rather than into the bar above it.
  const [editingInCell, setEditingInCell] = useState(false);

  const first = model.sheets[model.active];
  const [selected, setSelected] = useState<{ row: number; col: number }>({
    row: first?.firstRow ?? 0,
    col: first?.firstCol ?? 0,
  });
  // Where a range was started from, so shift-click and shift-arrow extend from the same corner.
  const [anchor, setAnchor] = useState<{ row: number; col: number } | undefined>();

  const sheet = model.sheets[active];
  const scroller = useRef<HTMLDivElement | null>(null);

  const cell = sheet ? cellAt(sheet, selected.row, selected.col) : undefined;
  const range: Range | undefined = anchor
    ? { fromRow: anchor.row, fromCol: anchor.col, toRow: selected.row, toCol: selected.col }
    : undefined;

  const rows = sheet?.rows.length ?? 0;
  const cols = sheet?.rows[0]?.length ?? 0;

  const move = useCallback(
    (downBy: number, acrossBy: number, extend: boolean) => {
      if (!sheet) return;
      const lastRow = sheet.firstRow + Math.max(rows - 1, 0);
      const lastCol = sheet.firstCol + Math.max(cols - 1, 0);
      const next = {
        row: Math.min(Math.max(selected.row + downBy, sheet.firstRow), lastRow),
        col: Math.min(Math.max(selected.col + acrossBy, sheet.firstCol), lastCol),
      };
      // Extending keeps the anchor where the selection started; moving plainly drops the range,
      // which is what every spreadsheet does and what the User's fingers expect.
      if (extend) {
        if (!anchor) setAnchor(selected);
      } else {
        setAnchor(undefined);
      }
      setTyped(undefined);
      setEditingInCell(false);
      setSelected(next);
    },
    [anchor, cols, rows, selected, sheet],
  );

  const commit = useCallback(
    (value: string, thenMove: "down" | "across" | "stay") => {
      if (!sheet || !onEdit) return;
      onEdit(sheet.name, cellRef(selected.row, selected.col), value);
      setTyped(undefined);
      setEditingInCell(false);
      // The input that took the typing is about to be unmounted. Without this the focus falls to
      // the document and the next key — an arrow, or undo — goes nowhere.
      scroller.current?.focus();
      if (thenMove === "down") move(1, 0, false);
      else if (thenMove === "across") move(0, 1, false);
    },
    [move, onEdit, selected, sheet],
  );

  /** Everything in the current selection, as rows of text. */
  const selectionAsText = useCallback((): string => {
    if (!sheet) return "";
    const box = normalise(
      range ?? {
        fromRow: selected.row,
        fromCol: selected.col,
        toRow: selected.row,
        toCol: selected.col,
      },
    );
    const lines: string[] = [];
    for (let row = box.fromRow; row <= box.toRow; row += 1) {
      const line: string[] = [];
      for (let col = box.fromCol; col <= box.toCol; col += 1) {
        // The value as read, not the formula. Copying a total and pasting it elsewhere should
        // give the total, which is what every other spreadsheet does with a plain copy.
        line.push(cellAt(sheet, row, col)?.display ?? "");
      }
      lines.push(line.join("\t"));
    }
    return lines.join("\n");
  }, [range, selected, sheet]);

  const paste = useCallback(
    (text: string) => {
      if (!sheet || !onEdit) return;
      // Tabs and newlines, which is what every spreadsheet and every text editor agrees on.
      const grid = text.replace(/\r\n?/g, "\n").replace(/\n$/, "").split("\n").map((line) => line.split("\t"));
      const changes: { cell: string; value: string }[] = [];
      grid.forEach((line, downBy) => {
        line.forEach((value, acrossBy) => {
          changes.push({
            cell: cellRef(selected.row + downBy, selected.col + acrossBy),
            value,
          });
        });
      });
      if (changes.length === 0) return;
      if (onEditMany) onEditMany(sheet.name, changes);
      else changes.forEach((change) => onEdit(sheet.name, change.cell, change.value));
    },
    [onEdit, onEditMany, selected, sheet],
  );

  // The keyboard, at the level of the whole grid rather than per cell: a spreadsheet's keyboard
  // belongs to the selection, not to whichever element happens to have focus.
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (!sheet) return;
    const meta = event.metaKey || event.ctrlKey;

    if (editingInCell) {
      if (event.key === "Enter") {
        event.preventDefault();
        commit(typed ?? "", "down");
      } else if (event.key === "Tab") {
        event.preventDefault();
        commit(typed ?? "", "across");
      } else if (event.key === "Escape") {
        event.preventDefault();
        setTyped(undefined);
        setEditingInCell(false);
      }
      return;
    }

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        move(1, 0, event.shiftKey);
        return;
      case "ArrowUp":
        event.preventDefault();
        move(-1, 0, event.shiftKey);
        return;
      case "ArrowLeft":
        event.preventDefault();
        move(0, -1, event.shiftKey);
        return;
      case "ArrowRight":
        event.preventDefault();
        move(0, 1, event.shiftKey);
        return;
      case "Tab":
        event.preventDefault();
        move(0, event.shiftKey ? -1 : 1, false);
        return;
      case "Enter":
        event.preventDefault();
        if (onEdit) {
          // Enter on a cell opens it for editing, holding what is already there, so a small
          // correction does not mean retyping the whole value.
          setTyped(cell?.formula ?? cell?.display ?? "");
          setEditingInCell(true);
        }
        return;
      case "Home":
        event.preventDefault();
        move(0, -1e6, event.shiftKey);
        return;
      case "End":
        event.preventDefault();
        move(0, 1e6, event.shiftKey);
        return;
      case "PageDown":
        event.preventDefault();
        move(20, 0, event.shiftKey);
        return;
      case "PageUp":
        event.preventDefault();
        move(-20, 0, event.shiftKey);
        return;
      case "Backspace":
      case "Delete": {
        if (!onEdit) return;
        event.preventDefault();
        // Clearing a range clears all of it, in one action.
        const box = normalise(
          range ?? {
            fromRow: selected.row,
            fromCol: selected.col,
            toRow: selected.row,
            toCol: selected.col,
          },
        );
        const cleared: { cell: string; value: string }[] = [];
        for (let row = box.fromRow; row <= box.toRow; row += 1) {
          for (let col = box.fromCol; col <= box.toCol; col += 1) {
            cleared.push({ cell: cellRef(row, col), value: "" });
          }
        }
        if (onEditMany) onEditMany(sheet.name, cleared);
        else cleared.forEach((one) => onEdit(sheet.name, one.cell, one.value));
        return;
      }
      default:
        break;
    }

    if (meta && (event.key === "z" || event.key === "Z") && onUndo) {
      event.preventDefault();
      onUndo();
      return;
    }

    if (meta && (event.key === "a" || event.key === "A")) {
      event.preventDefault();
      setAnchor({ row: sheet.firstRow, col: sheet.firstCol });
      setSelected({
        row: sheet.firstRow + Math.max(rows - 1, 0),
        col: sheet.firstCol + Math.max(cols - 1, 0),
      });
      return;
    }

    // A printable character starts an edit and becomes its first character, which is how a
    // spreadsheet lets you type over a cell without ceremony.
    if (!meta && onEdit && event.key.length === 1) {
      event.preventDefault();
      setTyped(event.key);
      setEditingInCell(true);
    }
  };

  // Focus put back after the remount an edit causes, and only if it was here before.
  useEffect(() => {
    if (wasWorkingHere) scroller.current?.focus();
  }, []);

  // Copy and paste are document-level events, so they are taken from the window while the grid
  // holds focus rather than from a hidden textarea.
  useEffect(() => {
    const holder = scroller.current;
    if (!holder) return;

    const onCopy = (event: ClipboardEvent) => {
      if (!holder.contains(document.activeElement)) return;
      event.preventDefault();
      event.clipboardData?.setData("text/plain", selectionAsText());
    };
    const onPaste = (event: ClipboardEvent) => {
      if (!holder.contains(document.activeElement)) return;
      const text = event.clipboardData?.getData("text/plain");
      if (!text) return;
      event.preventDefault();
      paste(text);
    };

    document.addEventListener("copy", onCopy);
    document.addEventListener("paste", onPaste);
    return () => {
      document.removeEventListener("copy", onCopy);
      document.removeEventListener("paste", onPaste);
    };
  }, [paste, selectionAsText]);

  const reference = !sheet
    ? ""
    : range
      ? `${cellRef(normalise(range).fromRow, normalise(range).fromCol)}:${cellRef(
          normalise(range).toRow,
          normalise(range).toCol,
        )}`
      : cellRef(selected.row, selected.col);

  // Told outward whenever it changes, so the toolbar is never acting on a stale selection.
  useEffect(() => {
    if (!sheet || !reference) return;
    const box = range
      ? normalise(range)
      : { fromRow: selected.row, fromCol: selected.col, toRow: selected.row, toCol: selected.col };
    onSelection?.({
      sheet: sheet.name,
      range: reference,
      // One-based, because everything outside the grid speaks in what the User sees.
      row: box.fromRow + 1,
      column: columnName(box.fromCol),
    });
  }, [onSelection, range, reference, selected, sheet]);

  if (!sheet) return null;

  return (
    // Three bands: the formula bar, the grid, and the sheets. The grid is the only one that
    // scrolls, so the reference and the sheet names stay put however far down the User is.
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
            minWidth: 62,
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
          }}
        >
          {reference}
        </span>
        {onEdit ? (
          <Field
            // Typing here is the User's own edit. It goes the same way an agent's does.
            value={typed ?? cell?.formula ?? cell?.display ?? ""}
            mono
            label={reference}
            onChange={(event) => setTyped(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") commit(typed ?? "", "down");
              else if (event.key === "Escape") setTyped(undefined);
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
        {range ? (
          <span className="hint" style={{ fontSize: 11 }}>
            {countOf(range)} cells
          </span>
        ) : null}
      </div>

      <div
        ref={scroller}
        // The grid takes the keyboard as a whole. Without a tabbable container the arrows would
        // scroll the pane instead of moving the selection.
        tabIndex={0}
        onFocus={() => {
          wasWorkingHere = true;
        }}
        onBlur={(event) => {
          // Still here if the focus went to a cell's own input.
          if (!event.currentTarget.contains(event.relatedTarget as Node)) wasWorkingHere = false;
        }}
        onKeyDown={onKeyDown}
        style={{ flex: 1, minHeight: 0, overflow: "auto", outline: "none" }}
      >
        <Grid
          sheet={sheet}
          selected={selected}
          range={range}
          editing={editingInCell ? (typed ?? "") : undefined}
          onTyping={setTyped}
          onCommit={commit}
          onCancel={() => {
            setTyped(undefined);
            setEditingInCell(false);
          }}
          onSelect={(at, extend) => {
            if (extend) {
              if (!anchor) setAnchor(selected);
            } else {
              setAnchor(undefined);
            }
            setTyped(undefined);
            setEditingInCell(false);
            setSelected(at);
            scroller.current?.focus();
          }}
          onOpen={(at) => {
            setSelected(at);
            setAnchor(undefined);
            if (!onEdit) return;
            const opening = cellAt(sheet, at.row, at.col);
            setTyped(opening?.formula ?? opening?.display ?? "");
            setEditingInCell(true);
          }}
          onSelectColumn={(col) => {
            setAnchor({ row: sheet.firstRow, col });
            setSelected({ row: sheet.firstRow + Math.max(rows - 1, 0), col });
            scroller.current?.focus();
          }}
          onSelectRow={(row) => {
            setAnchor({ row, col: sheet.firstCol });
            setSelected({ row, col: sheet.firstCol + Math.max(cols - 1, 0) });
            scroller.current?.focus();
          }}
        />

        {/* Below the grid rather than floating over the cells. The file records where a chart
            sits, but the reader does not yet give that position back, and drawing a chart at a
            guessed place would cover the User's own numbers. */}
        {sheet.charts && sheet.charts.length > 0 ? (
          <div style={{ display: "flex", gap: 14, flexWrap: "wrap", padding: "14px 2px" }}>
            {sheet.charts.map((chart, index) => (
              <ChartView key={`${chart.title ?? "chart"}-${index}`} chart={chart} />
            ))}
          </div>
        ) : null}
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
                    setAnchor(undefined);
                    setSelected({ row: candidate.firstRow, col: candidate.firstCol });
                  }}
                  aria-current={on ? "true" : undefined}
                  style={{
                    border: "1px solid",
                    borderColor: on ? "var(--border-strong)" : "transparent",
                    // The strip sits under the grid, so the active sheet opens upward into it
                    // rather than downward.
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

function countOf(range: Range): number {
  const box = normalise(range);
  return (box.toRow - box.fromRow + 1) * (box.toCol - box.fromCol + 1);
}

/** Which cells are covered by a merge, and so must not be drawn. */
function coveredBy(sheet: Sheet): Set<string> {
  const covered = new Set<string>();
  for (const merge of sheet.merges ?? []) {
    for (let row = merge.firstRow; row <= merge.lastRow; row += 1) {
      for (let col = merge.firstCol; col <= merge.lastCol; col += 1) {
        if (row === merge.firstRow && col === merge.firstCol) continue;
        covered.add(`${row}:${col}`);
      }
    }
  }
  return covered;
}

function Grid({
  sheet,
  selected,
  range,
  editing,
  onSelect,
  onOpen,
  onTyping,
  onCommit,
  onCancel,
  onSelectColumn,
  onSelectRow,
}: {
  sheet: Sheet;
  selected: { row: number; col: number };
  range?: Range;
  /** The text being typed into the selected cell, when the User is typing into the cell. */
  editing?: string;
  onSelect: (at: { row: number; col: number }, extend: boolean) => void;
  onOpen: (at: { row: number; col: number }) => void;
  onTyping: (value: string) => void;
  onCommit: (value: string, thenMove: "down" | "across" | "stay") => void;
  onCancel: () => void;
  onSelectColumn: (col: number) => void;
  onSelectRow: (row: number) => void;
}) {
  const cols = sheet.rows[0]?.length ?? 0;
  if (cols === 0) {
    return (
      <p className="hint" style={{ padding: 12 }}>
        This sheet is empty.
      </p>
    );
  }

  const covered = useMemo(() => coveredBy(sheet), [sheet]);

  // Column widths as the file has them. Excel measures a column in characters, so it is turned
  // into pixels here rather than in the Core, which reports what the file says.
  const widths = Array.from({ length: cols }, (_, index) => {
    const characters = sheet.columnWidths?.[index];
    return characters ? Math.max(Math.round(characters * 7.6), 34) : DEFAULT_COLUMN;
  });
  const template = `${ROW_HEADER}px ${widths.map((width) => `${width}px`).join(" ")}`;

  return (
    <div
      role="table"
      style={{
        background: "#fff",
        fontSize: 11.5,
        display: "inline-block",
        minWidth: "100%",
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
          // Held at the top so the letters are still there twenty rows down.
          position: "sticky",
          top: 0,
          zIndex: 3,
        }}
      >
        <div
          style={{
            padding: "5px 0",
            position: "sticky",
            left: 0,
            background: "#f7f6f3",
            zIndex: 4,
          }}
        />
        {Array.from({ length: cols }, (_, index) => {
          const absolute = sheet.firstCol + index;
          const on = range
            ? within(range, selected.row, absolute)
            : absolute === selected.col;
          return (
            <button
              key={index}
              type="button"
              role="columnheader"
              onClick={() => onSelectColumn(absolute)}
              style={{
                padding: "5px 0",
                border: 0,
                borderRight: "1px solid var(--border)",
                background: on ? "#e7e9ee" : "#f7f6f3",
                font: "inherit",
                fontWeight: 650,
                color: "inherit",
                cursor: "pointer",
              }}
            >
              {columnName(absolute)}
            </button>
          );
        })}
      </div>

      {sheet.rows.map((row, rowIndex) => {
        const absoluteRow = sheet.firstRow + rowIndex;
        const rowOn = range ? within(range, absoluteRow, selected.col) : absoluteRow === selected.row;
        return (
          <div
            key={absoluteRow}
            role="row"
            style={{ display: "grid", gridTemplateColumns: template, minHeight: ROW_HEIGHT }}
          >
            <button
              type="button"
              role="rowheader"
              onClick={() => onSelectRow(absoluteRow)}
              style={{
                padding: "6px 0",
                textAlign: "center",
                border: 0,
                borderRight: "1px solid var(--border)",
                background: rowOn ? "#e7e9ee" : "#f7f6f3",
                color: rowOn ? "var(--ink-soft)" : "var(--faint)",
                fontWeight: rowOn ? 650 : 400,
                font: "inherit",
                cursor: "pointer",
                // Held at the left, for the same reason the letters are held at the top.
                position: "sticky",
                left: 0,
                zIndex: 2,
              }}
            >
              {absoluteRow + 1}
            </button>
            {row.map((cell, colIndex) => {
              const absoluteCol = sheet.firstCol + colIndex;
              if (covered.has(`${absoluteRow}:${absoluteCol}`)) return null;
              const merge = (sheet.merges ?? []).find(
                (candidate) =>
                  candidate.firstRow === absoluteRow && candidate.firstCol === absoluteCol,
              );
              const isSelected = absoluteRow === selected.row && absoluteCol === selected.col;
              const inRange = range ? within(range, absoluteRow, absoluteCol) : false;
              return (
                <CellView
                  key={absoluteCol}
                  cell={cell}
                  selected={isSelected}
                  inRange={inRange && !isSelected}
                  span={
                    merge
                      ? {
                          across: merge.lastCol - merge.firstCol + 1,
                          down: merge.lastRow - merge.firstRow + 1,
                        }
                      : undefined
                  }
                  label={cellRef(absoluteRow, absoluteCol)}
                  editing={isSelected ? editing : undefined}
                  onTyping={onTyping}
                  onCommit={onCommit}
                  onCancel={onCancel}
                  onSelect={(extend) => onSelect({ row: absoluteRow, col: absoluteCol }, extend)}
                  onOpen={() => onOpen({ row: absoluteRow, col: absoluteCol })}
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
  inRange,
  span,
  label,
  editing,
  onSelect,
  onOpen,
  onTyping,
  onCommit,
  onCancel,
}: {
  cell: Cell;
  selected: boolean;
  inRange: boolean;
  /** How many columns and rows this cell covers, when the file merges it with its neighbours. */
  span?: { across: number; down: number };
  label: string;
  editing?: string;
  onSelect: (extend: boolean) => void;
  onOpen: () => void;
  onTyping: (value: string) => void;
  onCommit: (value: string, thenMove: "down" | "across" | "stay") => void;
  onCancel: () => void;
}) {
  const style = cell.style ?? {};
  const typing = editing !== undefined;

  return (
    <div
      role="cell"
      aria-label={`${label} ${cell.display}`}
      aria-selected={selected}
      onMouseDown={(event) => onSelect(event.shiftKey)}
      onDoubleClick={onOpen}
      style={{
        padding: typing ? 0 : "6px 9px",
        borderRight: "1px solid #f1efea",
        borderBottom: "1px solid #f1efea",
        gridColumn: span && span.across > 1 ? `span ${span.across}` : undefined,
        gridRow: span && span.down > 1 ? `span ${span.down}` : undefined,
        textAlign: style.align ?? (cell.numeric ? "right" : "left"),
        fontWeight: style.bold ? 650 : 400,
        fontStyle: style.italic ? "italic" : "normal",
        color: cell.error ? "var(--warn-fg)" : (style.colour ?? "#3d3933"),
        background: cell.error
          ? undefined
          : inRange
            ? "#eef2f7"
            : (style.background ?? "transparent"),
        outline: selected ? "2px solid #6f8fa8" : "none",
        outlineOffset: -2,
        cursor: "cell",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
      }}
      title={cell.error ? `That cell holds an error: ${cell.error}` : undefined}
    >
      {typing ? (
        <input
          // Autofocus is right here: the User has already started typing, and the caret has to
          // be where the characters are going.
          autoFocus
          value={editing}
          aria-label={label}
          onChange={(event) => onTyping(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onCommit(event.currentTarget.value, "down");
            } else if (event.key === "Tab") {
              event.preventDefault();
              onCommit(event.currentTarget.value, "across");
            } else if (event.key === "Escape") {
              event.preventDefault();
              onCancel();
            }
          }}
          style={{
            width: "100%",
            height: "100%",
            border: 0,
            outline: "none",
            padding: "6px 9px",
            font: "inherit",
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
            background: "#fff",
            textAlign: "left",
          }}
        />
      ) : (
        cell.display
      )}
    </div>
  );
}

function cellAt(sheet: Sheet, row: number, col: number): Cell | undefined {
  return sheet.rows[row - sheet.firstRow]?.[col - sheet.firstCol];
}
