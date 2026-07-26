/**
 * The spreadsheet contract.
 *
 * Mirrors `studio-sheets::GridModel` exactly. The Core reads the file with
 * `zavora-xlsx`, formats every value, and sends this; the renderer draws it and never
 * parses a spreadsheet itself.
 *
 * That is a deliberate departure from `excel-agent-app`, which fetches the `.xlsx`
 * and parses it in the browser with SheetJS. The reason is not tidiness: a
 * spreadsheet must have exactly one calculator. `zavora-xlsx` owns the formula engine
 * and writes the file, so if the renderer evaluated formulas too, the number on
 * screen and the number in the saved file could disagree — silently, in a financial
 * model, where being right is the entire value of the artefact.
 *
 * The cost is that presentation the Core does not yet extract cannot be shown. That
 * is the correct trade: a missing border is a cosmetic gap, a wrong total is a defect.
 */

export interface CellStyle {
  bold?: boolean;
  italic?: boolean;
  /** `#rrggbb` */
  colour?: string;
  background?: string;
  align?: "left" | "center" | "right";
}

export interface Cell {
  /** Already formatted by the Core. The renderer never formats a number. */
  display: string;
  /** With its leading `=`, for the formula bar. */
  formula?: string;
  /** True for numbers, so the grid can align them right. */
  numeric?: boolean;
  style?: CellStyle;
  /** Set when the cell holds an error, so it can be said plainly. */
  error?: string;
}

export interface Merge {
  firstRow: number;
  firstCol: number;
  lastRow: number;
  lastCol: number;
}

export interface Sheet {
  name: string;
  /** Zero-based position of the rectangle, so real row numbers can be shown. */
  firstRow: number;
  firstCol: number;
  rows: Cell[][];
  merges: Merge[];
  columnWidths: (number | null)[];
  /** The charts drawn on this sheet, with their numbers already resolved by the Core. */
  charts?: Chart[];
}

/** One series of a chart. A missing value is null, not zero: a gap is not a measurement. */
export interface ChartSeries {
  name?: string;
  labels: string[];
  values: (number | null)[];
}

export interface Chart {
  /** column, bar, line, pie, area, scatter — or "other" for one we cannot draw. */
  kind: string;
  title?: string;
  acrossName?: string;
  upName?: string;
  atRow: number;
  atCol: number;
  width: number;
  height: number;
  series: ChartSeries[];
}

export interface GridModel {
  fileName: string;
  sheets: Sheet[];
  active: number;
}

/** Spreadsheet column names: A, B, … Z, AA, AB. */
export function columnName(index: number): string {
  let name = "";
  let n = index;
  while (n >= 0) {
    name = String.fromCharCode(65 + (n % 26)) + name;
    n = Math.floor(n / 26) - 1;
  }
  return name;
}

/** `D7` for a cell, as the formula bar and the User both use. */
export function cellRef(row: number, col: number): string {
  return `${columnName(col)}${row + 1}`;
}

const bold: CellStyle = { bold: true };

/**
 * The fixture from the mockups, in the shape the Core sends.
 *
 * Note what it does *not* contain: no raw numbers awaiting formatting, and no
 * formula strings the renderer would have to evaluate. Both have already happened.
 */
export const SAMPLE_GRID: GridModel = {
  fileName: "Q3 revenue model.xlsx",
  active: 0,
  sheets: [
    {
      name: "Summary",
      firstRow: 4,
      firstCol: 0,
      merges: [],
      columnWidths: [14, 10, 14, 14],
      rows: [
        [
          { display: "Month", style: bold },
          { display: "Units", style: bold },
          { display: "Base", style: bold },
          { display: "+12%", style: { ...bold, background: "#eef4ea" } },
        ],
        [
          { display: "July" },
          { display: "1,240", numeric: true },
          { display: "4,960,000", numeric: true },
          {
            display: "5,555,200",
            numeric: true,
            formula: "=C6*1.12",
            style: { background: "#f7fbf5" },
          },
        ],
        [
          { display: "August" },
          { display: "1,310", numeric: true },
          { display: "5,240,000", numeric: true },
          { display: "5,868,800", numeric: true, formula: "=C7*1.12" },
        ],
        [
          { display: "September" },
          { display: "1,455", numeric: true },
          { display: "5,820,000", numeric: true },
          {
            display: "6,518,400",
            numeric: true,
            formula: "=C8*1.12",
            style: { background: "#f7fbf5" },
          },
        ],
        [
          { display: "Q3 total", style: bold },
          { display: "4,005", numeric: true, style: bold, formula: "=SUM(B6:B8)" },
          { display: "16,020,000", numeric: true, style: bold, formula: "=SUM(C6:C8)" },
          {
            display: "17,942,400",
            numeric: true,
            style: { ...bold, background: "#eef4ea" },
            formula: "=SUM(D6:D8)",
          },
        ],
      ],
    },
    { name: "Detail", firstRow: 0, firstCol: 0, rows: [], merges: [], columnWidths: [] },
    { name: "Assumptions", firstRow: 0, firstCol: 0, rows: [], merges: [], columnWidths: [] },
  ],
};
