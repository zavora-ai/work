/**
 * Agent fixtures.
 *
 * Shapes what the Agents section of Settings needs. Settings is where a User who has
 * gone looking for detail is allowed to find it (Requirement 23), so these fields are
 * deliberately more technical than anything on a primary surface.
 *
 * The quality figures are not a synthetic score. They come from what the User already
 * did in the tray: approving, editing before approving, excluding rows, rejecting,
 * and undoing afterwards. That is a graded human judgement the product collects
 * anyway, which is why it is the honest basis for quality over time.
 */

export type Tier = "Fast" | "Balanced" | "Best";

/** What an operation does to the world, as the side-effect gate classifies it. */
export type Effect = "reads" | "edits" | "acts";

export interface ToolGrant {
  name: string;
  what: string;
  effect: Effect;
  granted: boolean;
}

export interface MemoryFact {
  id: string;
  kind: "Profile" | "Lesson" | "Session";
  text: string;
  learnedFrom: string;
}

export interface Outcomes {
  approved: number;
  approvedWithEdits: number;
  rejected: number;
  undone: number;
}

export interface TelemetryGap {
  what: string;
  why: string;
  owner: string;
}

export interface AgentProfile {
  id: string;
  /** What it is, in the User's words. */
  name: string;
  does: string;
  tier: Tier;
  model: string;
  /** Its instructions, as shipped. */
  prompt: string;
  tools: ToolGrant[];
  shortTermTurns: number;
  compactAfter: number;
  facts: MemoryFact[];
  outcomes: Outcomes;
  /** Approval rate over the last 14 days, oldest first. */
  trend: number[];
  latencyP50: string;
  latencyP95: string;
  costPerDay: string;
  failoverRate: string;
  gaps: TelemetryGap[];
}

const SHARED_GAPS: TelemetryGap[] = [
  {
    what: "Cost before 19 July",
    why: "Spend was not recorded per piece of work until then",
    owner: "Work Studio",
  },
];

export const AGENTS: AgentProfile[] = [
  {
    id: "a-doc",
    name: "Document specialist",
    does: "Writes and edits documents, and co-edits them with you.",
    tier: "Balanced",
    model: "openai/gpt-5",
    prompt:
      "You write and revise documents for a professional who is not a writer by trade. Preserve the author's voice. Make the smallest change that satisfies the request. Never remove content the author did not ask you to remove. When a change affects a cross-reference, update it and say so.",
    tools: [
      { name: "read_document", what: "Read a document", effect: "reads", granted: true },
      { name: "edit_paragraph", what: "Change a paragraph", effect: "edits", granted: true },
      { name: "add_table", what: "Add a table", effect: "edits", granted: true },
      { name: "add_comment", what: "Leave a comment", effect: "edits", granted: true },
      { name: "export_pdf", what: "Save a PDF copy", effect: "edits", granted: true },
      { name: "send_email", what: "Email the document", effect: "acts", granted: false },
    ],
    shortTermTurns: 12,
    compactAfter: 20,
    facts: [
      {
        id: "m-1",
        kind: "Profile",
        text: "Our legal entity is Zavora Logistics Ltd, not Zavora Ltd.",
        learnedFrom: "You corrected me twice",
      },
      {
        id: "m-2",
        kind: "Lesson",
        text: "Contracts here put termination before confidentiality.",
        learnedFrom: "Three documents in a row",
      },
      {
        id: "m-3",
        kind: "Session",
        text: "Supplier agreement — you wanted 60 days' notice, not 30.",
        learnedFrom: "Yesterday",
      },
    ],
    outcomes: { approved: 34, approvedWithEdits: 11, rejected: 2, undone: 1 },
    trend: [68, 70, 72, 71, 75, 78, 77, 80, 82, 81, 84, 86, 85, 88],
    latencyP50: "4.1s",
    latencyP95: "11.7s",
    costPerDay: "$0.08",
    failoverRate: "0.4%",
    gaps: SHARED_GAPS,
  },
  {
    id: "a-deck",
    name: "Presentation specialist",
    does: "Builds and edits decks, and checks they read on a projector.",
    tier: "Balanced",
    model: "openai/gpt-5",
    prompt:
      "You build decks for board and client audiences. One idea per slide. Prefer a chart to a table and a table to a paragraph. Check contrast before calling a deck finished, and say what you fixed.",
    tools: [
      { name: "render_slide", what: "See a slide as it will look", effect: "reads", granted: true },
      { name: "add_shape", what: "Add or move a shape", effect: "edits", granted: true },
      { name: "add_chart", what: "Add a chart", effect: "edits", granted: true },
      { name: "apply_theme", what: "Apply your colours", effect: "edits", granted: true },
      { name: "lint_design", what: "Check it reads clearly", effect: "reads", granted: true },
    ],
    shortTermTurns: 10,
    compactAfter: 18,
    facts: [
      {
        id: "m-4",
        kind: "Profile",
        text: "Brand colours are deep green and sand. Never the default blue.",
        learnedFrom: "You told me on 14 July",
      },
      {
        id: "m-5",
        kind: "Lesson",
        text: "The ask goes on the last slide, not the first.",
        learnedFrom: "You reordered two decks",
      },
    ],
    outcomes: { approved: 21, approvedWithEdits: 9, rejected: 3, undone: 2 },
    trend: [55, 58, 60, 62, 61, 65, 68, 67, 70, 72, 74, 73, 76, 78],
    latencyP50: "6.8s",
    latencyP95: "19.2s",
    costPerDay: "$0.11",
    failoverRate: "1.1%",
    gaps: [
      ...SHARED_GAPS,
      {
        what: "Whether a deck was actually presented",
        why: "Nothing tells us what happens after you open it",
        owner: "Not planned",
      },
    ],
  },
  {
    id: "a-sheet",
    name: "Spreadsheet specialist",
    does: "Builds models, cleans data and writes formulas.",
    tier: "Best",
    model: "openai/gpt-5",
    prompt:
      "You build financial models for someone who will check your arithmetic. Every derived figure is a formula, never a pasted number. Label assumptions and keep them in one place. State the units.",
    tools: [
      { name: "read_sheet", what: "Read a sheet", effect: "reads", granted: true },
      { name: "set_cell", what: "Change a cell", effect: "edits", granted: true },
      { name: "add_formula", what: "Write a formula", effect: "edits", granted: true },
      { name: "add_pivot", what: "Add a pivot table", effect: "edits", granted: true },
      { name: "recalculate", what: "Recalculate", effect: "reads", granted: true },
    ],
    shortTermTurns: 14,
    compactAfter: 24,
    facts: [
      {
        id: "m-6",
        kind: "Profile",
        text: "Kenyan shillings, thousands separator, no decimals.",
        learnedFrom: "You reformatted a sheet",
      },
    ],
    outcomes: { approved: 40, approvedWithEdits: 6, rejected: 1, undone: 0 },
    trend: [80, 82, 81, 84, 85, 86, 88, 87, 90, 91, 92, 91, 93, 94],
    latencyP50: "3.2s",
    latencyP95: "8.9s",
    costPerDay: "$0.14",
    failoverRate: "0.2%",
    gaps: SHARED_GAPS,
  },
  {
    id: "a-proactive",
    name: "Proactive worker",
    does: "Runs the work you handed over, on its schedule.",
    tier: "Fast",
    model: "openai/gpt-5-mini",
    prompt:
      "You do recurring work unattended. Follow the instructions for this piece of work exactly. If you are unsure, stop and ask rather than guess. Never do anything outside this computer that you were not asked to do.",
    tools: [
      { name: "list_inbox", what: "Read the inbox", effect: "reads", granted: true },
      { name: "search_news", what: "Read your sources", effect: "reads", granted: true },
      { name: "move_to_folder", what: "Archive a message", effect: "acts", granted: true },
      { name: "send_email", what: "Send a message", effect: "acts", granted: true },
      { name: "post_to_x", what: "Post to X", effect: "acts", granted: true },
      { name: "delete_message", what: "Delete a message", effect: "acts", granted: false },
    ],
    shortTermTurns: 6,
    compactAfter: 10,
    facts: [
      {
        id: "m-7",
        kind: "Lesson",
        text: "Keep the newsletter under 400 words.",
        learnedFrom: "You shortened Friday's draft",
      },
      {
        id: "m-8",
        kind: "Lesson",
        text: "Receipts under £20 go straight to Expenses without asking.",
        learnedFrom: "You approved 12 in a row",
      },
    ],
    outcomes: { approved: 96, approvedWithEdits: 14, rejected: 4, undone: 3 },
    trend: [72, 74, 76, 75, 78, 80, 82, 81, 83, 85, 86, 88, 87, 89],
    latencyP50: "2.4s",
    latencyP95: "6.1s",
    costPerDay: "$0.29",
    failoverRate: "2.3%",
    gaps: [
      ...SHARED_GAPS,
      {
        what: "Whether a newsletter was read",
        why: "We do not track opens, and will not",
        owner: "By design",
      },
    ],
  },
];

/** Approval rate: the share of work accepted without correction. */
export function approvalRate(outcomes: Outcomes): number {
  const total =
    outcomes.approved + outcomes.approvedWithEdits + outcomes.rejected;
  return total === 0 ? 0 : Math.round((outcomes.approved / total) * 100);
}

export function totalDecisions(outcomes: Outcomes): number {
  return outcomes.approved + outcomes.approvedWithEdits + outcomes.rejected;
}
