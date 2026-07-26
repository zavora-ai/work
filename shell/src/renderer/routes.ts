/**
 * Where the interface can be.
 *
 * Deliberately flat and small. Every route is a place the User recognises, and
 * there is no route that exists only for the product's own machinery.
 */

export type Route =
  | "firstRun"
  | "dashboard"
  | "tray"
  | "outTray"
  | "thread"
  | "kickoffOutput"
  | "kickoffManifest"
  | "new"
  | "library"
  | "repository"
  | "documents"
  | "decks"
  | "spreadsheets"
  | "documentWorkspace"
  | "spreadsheetWorkspace"
  | "deckWorkspace"
  | "limits"
  | "settings"
  | "agents"
  | "privacy"
  | "diagnostics"
  | "states";

/** For the review-and-screenshot pass; not a User-facing surface. */
export const ALL_ROUTES: { route: Route; label: string }[] = [
  { route: "firstRun", label: "First run" },
  { route: "dashboard", label: "Dashboard" },
  { route: "tray", label: "Waiting on you" },
  { route: "outTray", label: "Done for you" },
  { route: "thread", label: "Thread detail" },
  { route: "kickoffOutput", label: "First draft — output" },
  { route: "kickoffManifest", label: "First draft — actions" },
  { route: "new", label: "New work" },
  { route: "library", label: "Recurring library" },
  { route: "repository", label: "Documents" },
  { route: "documentWorkspace", label: "Document workspace" },
  { route: "spreadsheetWorkspace", label: "Spreadsheet workspace" },
  { route: "deckWorkspace", label: "Deck workspace" },
  { route: "limits", label: "Honest limits" },
  { route: "settings", label: "Settings" },
  { route: "agents", label: "Agents" },
  { route: "privacy", label: "Privacy" },
  { route: "diagnostics", label: "Technical details" },
  { route: "states", label: "States" },
];
