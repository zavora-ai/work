/**
 * Fixture data.
 *
 * Stands in for the Core until the interface is wired to it. Shapes mirror the
 * view types in `core/crates/studio-api`, so wiring is a matter of replacing the
 * source rather than reworking components.
 *
 * Content is taken from the journeys in `.kiro/specs/zavora-work-studio/journeys.md`
 * so that the screens can be read against them.
 */

export type StateBadge = "working" | "scheduled" | "needsYou" | "finished" | "paused";

export interface Thread {
  id: string;
  purpose: string;
  badge: StateBadge;
  /** The concrete fact behind the badge, for hover, focus and accessible name. */
  statusDetail: string;
  scheduleHuman?: string;
  nextHuman?: string;
  spendToday?: string;
}

export const THREADS: Thread[] = [
  {
    id: "t-newsletter",
    purpose: "Daily newsletter",
    badge: "scheduled",
    statusDetail: "Next tomorrow, 7:00 am",
    scheduleHuman: "Every weekday at 7:00 am",
    nextHuman: "Tomorrow, 7:00 am",
    spendToday: "4p a day",
  },
  {
    id: "t-triage",
    purpose: "Inbox triage",
    badge: "needsYou",
    statusDetail: "Gmail needs reconnecting",
  },
  {
    id: "t-health",
    purpose: "Computer health",
    badge: "working",
    statusDetail: "Checking now · every 2 hours",
  },
  {
    id: "t-deck",
    purpose: "Board deck — July",
    badge: "finished",
    statusDetail: "Finished 21:04",
  },
  {
    id: "t-model",
    purpose: "Q3 revenue model",
    badge: "finished",
    statusDetail: "Finished yesterday",
  },
  {
    id: "t-agreement",
    purpose: "Partnership agreement",
    badge: "working",
    statusDetail: "You edited it 2 days ago",
  },
];

export type TrayClass = "kickoff" | "escalation" | "finding" | "attention";

export interface TrayItem {
  id: string;
  cls: TrayClass;
  headline: string;
  detail: string;
  choices: string[];
}

export const TRAY: TrayItem[] = [
  {
    id: "i-1",
    cls: "kickoff",
    headline: "Your daily newsletter is ready",
    detail: "Nothing has been sent yet · Daily newsletter",
    choices: ["Read it"],
  },
  {
    id: "i-2",
    cls: "escalation",
    headline: "Two of these look like the same invoice",
    detail:
      "I was filing receipts and found a £480 charge twice, three days apart. Expense capture.",
    choices: ["File both", "File one"],
  },
  {
    id: "i-3",
    cls: "finding",
    headline: "Your startup disk is 94% full",
    detail:
      "Nothing is broken yet. 18 GB sits in Downloads from before April. Computer health.",
    choices: ["See what's big", "Got it"],
  },
  {
    id: "i-4",
    cls: "attention",
    headline: "Gmail needs reconnecting",
    detail:
      "Your sign-in expired on Thursday. Inbox triage and Morning digest are paused until you reconnect — I've stopped trying.",
    choices: ["Reconnect"],
  },
];

export interface Delivery {
  id: string;
  action: string;
  when: string;
  thread: string;
  reversal: { kind: "available"; label: string; note?: string } | { kind: "none"; reason: string };
  extra?: string;
}

export const DELIVERIES: Delivery[] = [
  {
    id: "d-1",
    action: 'Posted to X: "We shipped local-first document editing…"',
    when: "9:02 am",
    thread: "Social posting",
    reversal: { kind: "available", label: "Take it down", note: "for 30 days" },
    extra: "14 likes so far",
  },
  {
    id: "d-2",
    action: "Sent your morning digest",
    when: "7:00 am",
    thread: "Morning digest",
    reversal: { kind: "none", reason: "Can't be unsent" },
    extra: "4 things needed you",
  },
  {
    id: "d-3",
    action: "Filed 3 receipts into Expenses.xlsx",
    when: "6:40 am",
    thread: "Expense capture",
    reversal: { kind: "available", label: "Undo" },
  },
  {
    id: "d-4",
    action: "Checked your computer — all clear",
    when: "6:00 am",
    thread: "Computer health",
    reversal: { kind: "none", reason: "Nothing to undo" },
    extra: "quiet checks are hidden",
  },
  {
    id: "d-5",
    action: "Archived 18 messages, drafted 4 replies",
    when: "Yesterday, 5:00 pm",
    thread: "Inbox triage",
    reversal: { kind: "available", label: "Undo all" },
  },
];

export interface SteeringNote {
  id: string;
  note: string;
  provenance: string;
  scope?: "Everything" | "Documents" | "Decks" | "Spreadsheets";
}

export const THREAD_NOTES: SteeringNote[] = [
  {
    id: "n-1",
    note: "Keep it under 400 words.",
    provenance: "You shortened Friday's draft · 2 days ago",
  },
  { id: "n-2", note: "Don't include crypto prices.", provenance: "You told me on Friday" },
  {
    id: "n-3",
    note: "Lead with anything about the EU AI Act.",
    provenance: "You told me last week",
  },
];

export const GLOBAL_NOTES: SteeringNote[] = [
  {
    id: "g-1",
    note: 'Write plainly. No exclamation marks, no "excited to share".',
    provenance: "You told me on 12 July",
    scope: "Everything",
  },
  {
    id: "g-2",
    note: "Use our brand colours — deep green and sand. Never the default blue.",
    provenance: "You told me on 14 July",
    scope: "Decks",
  },
  {
    id: "g-3",
    note: "Put the ask on the last slide, not the first.",
    provenance: "You reordered two decks · 18 July",
    scope: "Decks",
  },
  {
    id: "g-4",
    note: "Kenyan shillings, thousands separator, no decimals.",
    provenance: "You reformatted a sheet · 19 July",
    scope: "Spreadsheets",
  },
  {
    id: "g-5",
    note: "Our legal entity is Zavora Logistics Ltd, not Zavora Ltd.",
    provenance: "You corrected me twice",
    scope: "Documents",
  },
];

export const RUN_HISTORY = [
  { text: "Sent your Monday brief — 3 sources, 6 minute read", when: "Today, 7:00 am" },
  { text: "Sent Friday's brief — you said it was too long", when: "Friday, 7:00 am" },
  { text: "Sent Thursday's brief", when: "Thursday, 7:00 am" },
  { text: "You approved the first draft", when: "Wednesday, 4:12 pm" },
];

export interface Template {
  id: string;
  name: string;
  what: string;
  needs?: string;
}

export const TEMPLATES: Template[] = [
  {
    id: "tpl-news",
    name: "Daily newsletter",
    what: "A short brief from your sources, in your inbox each morning.",
  },
  {
    id: "tpl-triage",
    name: "Inbox triage",
    what: "Sorts, flags and drafts replies. You keep the send button.",
    needs: "Gmail",
  },
  {
    id: "tpl-health",
    name: "Computer health",
    what: "Watches disk, memory and backups. Tells you before it bites.",
  },
];

export const MORE_TEMPLATES: Template[] = [
  { id: "tpl-digest", name: "Morning digest", what: "", needs: "Gmail · Calendar" },
  { id: "tpl-meeting", name: "Meeting prep", what: "", needs: "Calendar" },
  { id: "tpl-social", name: "Social posting", what: "", needs: "X" },
  { id: "tpl-report", name: "Weekly report roll-up", what: "" },
  { id: "tpl-expenses", name: "Expense capture", what: "", needs: "Gmail" },
  { id: "tpl-news-watch", name: "News and competitor monitor", what: "" },
  { id: "tpl-site", name: "Website availability", what: "" },
];

export interface FileRow {
  id: string;
  name: string;
  kind: "folder" | "document" | "deck" | "sheet";
  sub?: string;
  changed: string;
  changedBy?: string;
  usedIn: string[];
  versions?: number;
}

export const FILES: FileRow[] = [
  {
    id: "f-boardpacks",
    name: "Board packs",
    kind: "folder",
    sub: "3 files",
    changed: "Yesterday",
    usedIn: [],
  },
  {
    id: "f-expenses",
    name: "Expenses",
    kind: "folder",
    sub: "6 files · filled by Expense capture",
    changed: "6:40 am",
    usedIn: ["Expense capture"],
  },
  {
    id: "f-model",
    name: "Q3 revenue model.xlsx",
    kind: "sheet",
    sub: "You and I both edited it",
    changed: "20 min ago",
    changedBy: "by you",
    usedIn: ["Q3 revenue model", "Board deck — July"],
    versions: 12,
  },
  {
    id: "f-deck",
    name: "Board deck — July.pptx",
    kind: "deck",
    sub: "Made from Q3 revenue model.xlsx",
    changed: "21:04",
    changedBy: "by me",
    usedIn: ["Board deck — July"],
    versions: 4,
  },
  {
    id: "f-agreement",
    name: "Partnership agreement — draft 4.docx",
    kind: "document",
    sub: "Copy of draft 3, which kept your lawyer's markup",
    changed: "2 days ago",
    changedBy: "by you, in Word",
    usedIn: ["Partnership agreement"],
    versions: 7,
  },
  {
    id: "f-weekly",
    name: "Weekly report — 19 Jul.docx",
    kind: "document",
    sub: "Written by Weekly report roll-up",
    changed: "Sun 08:00",
    changedBy: "by me",
    usedIn: ["Weekly report roll-up"],
    versions: 1,
  },
];

/**
 * The figures on the Dashboard.
 *
 * These were `5`, `3`, `11` and `$0.62` — invented, and indistinguishable on screen from
 * counts that had been measured. A figure we do not have is reported as unavailable rather
 * than as zero or as a plausible number, because a wrong figure shown confidently is worse
 * than an absent one: the User cannot tell which they are looking at.
 *
 * They become real when the store is asked for them. Until then the em dash is the honest
 * answer.
 */
export const UNAVAILABLE = "—";

export const METRICS = {
  working: UNAVAILABLE,
  waiting: UNAVAILABLE,
  done: UNAVAILABLE,
  cost: UNAVAILABLE,
};
