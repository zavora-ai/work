/**
 * The application shell.
 *
 * Holds which surface is showing, which pane the right panel is on, and whether the
 * rails are collapsed. Nothing here is authoritative: when the interface is wired to
 * the Core, state arrives over the event stream and this file changes very little.
 *
 * The screen switcher at the bottom exists for review and screenshotting. It is
 * development-only and is not a User-facing surface.
 */

import { useState } from "react";

import { t } from "../shared/strings.ts";
import { TRAY } from "./fixtures.ts";
import { LeftPanel, type Navigator } from "./components/LeftPanel.tsx";
import type { Pane } from "./components/Workspace.tsx";
import { Dashboard } from "./screens/Dashboard.tsx";
import { KickoffManifest, KickoffOutput } from "./screens/Kickoff.tsx";
import { FirstRun, NewWork, RecurringLibrary } from "./screens/Onboarding.tsx";
import { Repository } from "./screens/Repository.tsx";
import { Diagnostics } from "./screens/Diagnostics.tsx";
import { Settings } from "./screens/Settings.tsx";
import { StateGallery } from "./screens/StateGallery.tsx";
import { InTray, OutTray, ThreadDetail } from "./screens/Trays.tsx";
import {
  DeckWorkspace,
  DocumentWorkspace,
  HonestLimits,
  SpreadsheetWorkspace,
} from "./screens/Workspaces.tsx";
import { ALL_ROUTES, type Route } from "./routes.ts";
import { useDeck, useDocument, type DeckState, type DocumentState } from "./useArtefact.ts";

/**
 * The route switcher is a review tool, not part of the product.
 *
 * It was visible in the built application, which is how a reviewer's convenience becomes a
 * User's confusion. It now appears only when the interface is being reviewed in a browser,
 * never in the desktop application, where `window.studio` exists.
 */
const REVIEW_MODE = typeof window !== "undefined" && window.studio === undefined;

const WORKSPACE_ROUTES: Route[] = [
  "documentWorkspace",
  "spreadsheetWorkspace",
  "deckWorkspace",
  "limits",
];

const NAVIGATORS: Partial<Record<Route, Navigator>> = {
  // documentWorkspace and deckWorkspace are derived from the open file instead, by
  // `navigatorFor` below. A fixed list here would contradict the canvas as soon as a real
  // file was opened, which is exactly the bug it caused.
  limits: {
    label: "In this document",
    items: [
      { label: "7. Payment terms" },
      { label: "8. Termination", on: true },
      { label: "9. Confidentiality" },
    ],
  },
  // No entry for spreadsheetWorkspace: the sheets are on the grid's bottom edge, where a
  // spreadsheet keeps them. Listing them again on the left duplicated the control and,
  // because the list was written here rather than read from the file, it also named
  // sheets the open file did not have.
};

/**
 * The navigator for a workspace, from the file it has open.
 *
 * One model feeds both this and the canvas, so what the sidebar lists is always what the
 * canvas is drawing. When there is no file, the sample's own structure is listed, so the
 * two still agree.
 */
function navigatorFor(
  route: Route,
  doc: DocumentState,
  deck: DeckState,
  activeSlide: number,
): Navigator | undefined {
  if (route === "documentWorkspace") {
    const outline = doc.model?.outline ?? [];
    return {
      label: t("doc.in_this_document"),
      items:
        outline.length === 0
          ? [{ label: t("doc.no_headings") }]
          : outline.map((entry) => ({
              label: entry.text,
              indent: entry.level > 1,
            })),
    };
  }
  if (route === "deckWorkspace") {
    const slides = deck.model?.slides ?? [];
    return {
      label: t("doc.slides"),
      items: slides.map((slide, index) => ({
        label: slide.title,
        badge: String(slide.number),
        on: index === activeSlide,
      })),
    };
  }
  return NAVIGATORS[route];
}

export function App() {
  // A file named at launch opens in its own workspace; otherwise the Dashboard, which is
  // what someone opening the app without a file in mind wants to see.
  const [route, setRoute] = useState<Route>(() => {
    if (typeof window === "undefined") return "dashboard";
    const asked = new URLSearchParams(window.location.search);
    if (asked.get("sheet")) return "spreadsheetWorkspace";
    if (asked.get("document")) return "documentWorkspace";
    if (asked.get("deck")) return "deckWorkspace";
    return "dashboard";
  });
  const [threadId, setThreadId] = useState<string | undefined>();
  const [leftCollapsed, setLeftCollapsed] = useState(false);
  const [rightCollapsed, setRightCollapsed] = useState(false);
  const [pane, setPane] = useState<Pane>("chat");
  // Which file each workspace is looking at.
  //
  // A real path makes the workspace ask the Core; without one the sample stands in and
  // says so. During review the path can be given in the address, which is how a screen
  // can be driven against a real file without an Electron window.
  const [paths, setPaths] = useState<{ sheet?: string; document?: string; deck?: string }>(() => {
    if (typeof window === "undefined") return {};
    const asked = new URLSearchParams(window.location.search);
    return {
      sheet: asked.get("sheet") ?? undefined,
      document: asked.get("document") ?? undefined,
      deck: asked.get("deck") ?? undefined,
    };
  });

  // Loaded once here so the sidebar and the canvas cannot disagree about the same file.
  const doc = useDocument(paths.document);
  const deck = useDeck(paths.deck);
  const [activeSlide, setActiveSlide] = useState(0);
  // Bumped when a file changed, so the work list and folder listings refetch.
  const [conversationChangedAt, setConversationChangedAt] = useState(0);

  /**
   * Open one of the User's own files in the workspace that understands it.
   *
   * The kind decides the workspace, so the User picks a file rather than picking a
   * workspace and then a file. An unknown kind is not an error worth a dialog: there is
   * simply nothing here that can open it.
   */
  /** Open one of the User's own files, by path. */
  const openPath = (path: string, kind?: string) => {
    const extension = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
    const which = kind ?? extension;
    if (which === "spreadsheet" || extension === "xlsx") {
      setPaths((current) => ({ ...current, sheet: path }));
      navigate("spreadsheetWorkspace");
    } else if (which === "document" || extension === "docx") {
      setPaths((current) => ({ ...current, document: path }));
      navigate("documentWorkspace");
    } else if (which === "deck" || extension === "pptx") {
      setPaths((current) => ({ ...current, deck: path }));
      navigate("deckWorkspace");
    }
  };

  const openFile = async () => {
    const bridge = typeof window === "undefined" ? undefined : window.studio;
    const chosen = await bridge?.openFile?.();
    if (!chosen) return;
    const extension = chosen.slice(chosen.lastIndexOf(".") + 1).toLowerCase();
    if (extension === "xlsx") {
      setPaths((current) => ({ ...current, sheet: chosen }));
      navigate("spreadsheetWorkspace");
    } else if (extension === "docx") {
      setPaths((current) => ({ ...current, document: chosen }));
      navigate("documentWorkspace");
    } else if (extension === "pptx") {
      setPaths((current) => ({ ...current, deck: chosen }));
      navigate("deckWorkspace");
    }
  };

  const navigate = (next: Route, id?: string) => {
    setRoute(next);
    if (id) setThreadId(id);
    if (WORKSPACE_ROUTES.includes(next)) setPane("chat");
  };

  // First run has no chrome: there is nothing to navigate yet.
  if (route === "firstRun") {
    return (
      <div className="app">
        <FirstRun onNavigate={navigate} />
        {REVIEW_MODE ? <Switcher route={route} onSelect={navigate} /> : null}
      </div>
    );
  }

  const workspace = WORKSPACE_ROUTES.includes(route);
  const workspaceProps = {
    pane,
    onPane: setPane,
    rightCollapsed,
    onToggleRight: () => setRightCollapsed((value) => !value),
  };

  return (
    <div className="app">
      <SkipLink />
      <LeftPanel
        route={route}
        onNavigate={navigate}
        collapsed={leftCollapsed}
        onToggle={() => setLeftCollapsed((value) => !value)}
        waitingCount={TRAY.length}
        // Clicking a piece of work reopens the file it was about, which is what returning
        // to it means. The list is refetched whenever a file changed.
        onOpenThread={(thread) => {
          if (thread.file) openPath(thread.file);
          else navigate("thread", thread.id);
        }}
        threadsChangedAt={conversationChangedAt}
        navigator={navigatorFor(route, doc, deck, activeSlide)}
      />

      {workspace ? (
        route === "documentWorkspace" ? (
          <DocumentWorkspace {...workspaceProps} state={doc} />
        ) : route === "spreadsheetWorkspace" ? (
          <SpreadsheetWorkspace {...workspaceProps} path={paths.sheet} thread={threadFor(paths.sheet)} />
        ) : route === "deckWorkspace" ? (
          <DeckWorkspace {...workspaceProps} state={deck} active={activeSlide} onActive={setActiveSlide} />
        ) : (
          <HonestLimits {...workspaceProps} />
        )
      ) : (
        <Screen route={route} threadId={threadId} onNavigate={navigate} openFile={openFile} openPath={openPath} />
      )}

      {REVIEW_MODE ? <Switcher route={route} onSelect={navigate} /> : null}
    </div>
  );
}

function SkipLink() {
  return (
    <button
      type="button"
      className="skip"
      onClick={() => {
        const main = document.querySelector(".main");
        if (main instanceof HTMLElement) {
          main.setAttribute("tabindex", "-1");
          main.focus();
        }
      }}
    >
      {t("nav.skip")}
    </button>
  );
}

/**
 * Which piece of work a file belongs to.
 *
 * One identifier per file, so opening a different spreadsheet is a different piece of work
 * with its own conversation and its own notes. Everything shared a single identifier before,
 * which merged every file the User had ever opened into one.
 */
function threadFor(path?: string): string | undefined {
  if (!path) return undefined;
  return `file:${path}`;
}

function Screen({
  route,
  threadId,
  onNavigate,
  openFile,
  openPath,
}: {
  route: Route;
  threadId?: string;
  onNavigate: (route: Route, threadId?: string) => void;
  /** Ask the User for a file and open it. Absent outside the desktop app. */
  openFile?: () => void;
  /** Open one of the User's own files, by path. */
  openPath?: (path: string, kind?: string) => void;
}) {
  switch (route) {
    case "dashboard":
      return <Dashboard onNavigate={onNavigate} />;
    case "tray":
      return <InTray onNavigate={onNavigate} />;
    case "outTray":
      return <OutTray />;
    case "thread":
      return <ThreadDetail threadId={threadId} />;
    case "kickoffOutput":
      return <KickoffOutput />;
    case "kickoffManifest":
      return <KickoffManifest />;
    case "new":
      return <NewWork onNavigate={onNavigate} onOpenFile={openFile} />;
    case "library":
      return <RecurringLibrary onNavigate={onNavigate} />;
    case "repository":
    case "documents":
    case "decks":
    case "spreadsheets":
      return <Repository key={route} onNavigate={onNavigate} route={route} onOpenFile={openFile} onOpenPath={openPath} />;
    // Keyed so that arriving from a different route resets which section is open:
    // without it React reuses the component and its tab state persists.
    case "settings":
      return <Settings key="settings" onDiagnostics={() => onNavigate("diagnostics")} />;
    case "agents":
      return <Settings key="agents" initialTab="Agents" />;
    case "privacy":
      return <Settings key="privacy" initialTab="Privacy" />;
    case "diagnostics":
      return <Diagnostics />;
    case "states":
      return <StateGallery />;
    default:
      return <NotBuiltYet route={route} />;
  }
}

function NotBuiltYet({ route }: { route: Route }) {
  return (
    <div className="main">
      <h1 className="h1">Not built yet</h1>
      <p className="hint">This surface is next: {route}</p>
    </div>
  );
}

function Switcher({ route, onSelect }: { route: Route; onSelect: (route: Route) => void }) {
  return (
    <div
      style={{
        position: "fixed",
        bottom: 10,
        left: "50%",
        transform: "translateX(-50%)",
        display: "flex",
        gap: 4,
        flexWrap: "wrap",
        justifyContent: "center",
        maxWidth: "78vw",
        background: "rgba(28,27,25,.9)",
        borderRadius: 20,
        padding: "6px 10px",
        zIndex: 100,
      }}
    >
      {ALL_ROUTES.map((entry) => (
        <button
          key={entry.route}
          type="button"
          onClick={() => onSelect(entry.route)}
          style={{
            border: 0,
            background: entry.route === route ? "#fff" : "transparent",
            color: entry.route === route ? "#1c1b19" : "rgba(255,255,255,.72)",
            fontSize: 11,
            fontWeight: 600,
            padding: "4px 9px",
            borderRadius: 14,
            cursor: "pointer",
          }}
        >
          {entry.label}
        </button>
      ))}
    </div>
  );
}
