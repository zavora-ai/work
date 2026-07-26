/**
 * The left panel.
 *
 * One persistent panel across the whole product: New work, Dashboard, the unified
 * thread list, the Documents repository, and Settings at the foot. There are no
 * separate destinations for documents and proactive work, because a thread *is* the
 * destination.
 *
 * `Your work` holds both Job kinds in one list. A scheduled piece of work and a
 * one-off differ only by their status indicator, never by section (Requirement 3.2).
 *
 * Collapsible to a narrow strip that keeps the status glyphs visible, so the User
 * can still see that something needs them while focusing on a document.
 */

import { t } from "../../shared/strings.ts";
import { useThreads, type Thread } from "../useOwn.ts";
import type { Route } from "../routes.ts";
import { Icon, SectionLabel } from "./primitives.tsx";
import { StatusGlyph } from "./state.tsx";

const DOC_ENTRIES: { route: Route; label: string; icon: Parameters<typeof Icon>[0]["name"] }[] = [
  { route: "repository", label: t("repo.all_files"), icon: "folder" },
  { route: "documents", label: t("repo.kind.documents"), icon: "document" },
  { route: "decks", label: t("repo.kind.decks"), icon: "deck" },
  { route: "spreadsheets", label: t("repo.kind.spreadsheets"), icon: "sheet" },
];

/**
 * When a file is open, the panel also carries that file's own navigator — an
 * outline for a document, thumbnails for a deck, sheets for a spreadsheet — which
 * is where office applications have always put it. The Documents repository
 * collapses to a single row so the navigator has room: the panel is contextual, not
 * fixed.
 */
/** How a piece of work reads when hovered: the file it is about, and when it changed. */
function describeThread(thread: Thread): string {
  const name = thread.file?.split("/").pop();
  return name ? `${name} · ${whenChanged(thread.changed)}` : whenChanged(thread.changed);
}

function whenChanged(seconds: number): string {
  const minutes = Math.floor((Date.now() - seconds * 1000) / 60000);
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes} min ago`;
  if (minutes < 60 * 24) return `${Math.floor(minutes / 60)} hours ago`;
  return new Date(seconds * 1000).toLocaleDateString([], { day: "numeric", month: "short" });
}

export interface Navigator {
  label: string;
  items: { label: string; on?: boolean; indent?: boolean; badge?: string }[];
}

/// How much room the window's own controls need at the top left.
///
/// The window is drawn with its title bar hidden, which is what gives the product the whole
/// surface — but the close, minimise and zoom controls are still there, floating over whatever
/// the interface puts underneath. The app name was landing at 20px from the top and 12 from the
/// left, directly under them.
///
/// Reserved rather than nudged: the controls do not move, so the space they need is a fact about
/// the window, not a matter of taste. 38 clears them with the margin macOS applications normally
/// leave — 30 cleared them by about three pixels, which reads as a near miss rather than a
/// decision.
const WINDOW_CONTROLS = 38;

/// The strip above everything, kept clear for the window's controls.
///
/// It is also the only place the window can be dragged by. Hiding the title bar removes the
/// usual grab area, so without this the window could only be moved by the narrow margin macOS
/// leaves beside the controls.
function ControlsRoom() {
  return (
    <div
      aria-hidden="true"
      style={
        {
          height: WINDOW_CONTROLS,
          flex: "0 0 auto",
          WebkitAppRegion: "drag",
        } as React.CSSProperties
      }
    />
  );
}

export function LeftPanel({
  route,
  onNavigate,
  collapsed,
  onToggle,
  waitingCount,
  navigator,
  onOpenThread,
  threadsChangedAt,
}: {
  route: Route;
  onNavigate: (route: Route, threadId?: string) => void;
  collapsed: boolean;
  onToggle: () => void;
  waitingCount: number;
  navigator?: Navigator;
  /** Open one of the User's own pieces of work. */
  onOpenThread?: (thread: Thread) => void;
  /** Bumped when the work list should be refetched. */
  threadsChangedAt?: number;
}) {
  const { threads } = useThreads(threadsChangedAt);

  if (collapsed) {
    return (
      <nav
        aria-label={t("nav.your_work")}
        style={{
          width: 44,
          background: "var(--card)",
          borderRight: "1px solid var(--border)",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          paddingTop: 0,
          gap: 14,
          flex: "0 0 auto",
        }}
      >
        <ControlsRoom />
        <button
          type="button"
          onClick={onToggle}
          title={t("nav.expand")}
          aria-label={t("nav.expand")}
          style={{ border: 0, background: "none", cursor: "pointer", color: "#8a8580" }}
        >
          <Icon name="chevronRight" size={16} />
        </button>
        {threads.slice(0, 4).map((thread) => (
          <StatusGlyph key={thread.id} badge="finished" detail={thread.purpose} />
        ))}
      </nav>
    );
  }

  return (
    <nav
      aria-label={t("nav.your_work")}
      style={{
        width: 206,
        background: "var(--card)",
        borderRight: "1px solid var(--border)",
        padding: "0 12px 20px",
        display: "flex",
        flexDirection: "column",
        flex: "0 0 auto",
        overflowY: "auto",
      }}
    >
      <ControlsRoom />
      <div
        style={{
          fontWeight: 650,
          fontSize: 14.5,
          padding: "0 8px 18px",
          letterSpacing: "-.01em",
          display: "flex",
          alignItems: "center",
        }}
      >
        Zavora Work Studio
        <button
          type="button"
          onClick={onToggle}
          title={t("nav.collapse")}
          aria-label={t("nav.collapse")}
          className="ml"
          style={{ border: 0, background: "none", cursor: "pointer", color: "#c4bfb6" }}
        >
          <Icon name="chevronLeft" size={14} />
        </button>
      </div>

      <NavRow
        icon="plus"
        label={t("nav.new_work")}
        on={route === "new"}
        onClick={() => onNavigate("new")}
      />
      <NavRow
        icon="dashboard"
        label={t("nav.dashboard")}
        on={route === "dashboard"}
        badge={waitingCount}
        onClick={() => onNavigate("dashboard")}
      />

      <SectionLabel>{t("nav.your_work")}</SectionLabel>
      <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
        {threads.length === 0 ? (
          <p
            style={{
              fontSize: 11.5,
              color: "var(--muted)",
              margin: "2px 9px 6px",
              lineHeight: 1.5,
            }}
          >
            {t("nav.no_work_yet")}
          </p>
        ) : null}
        {threads.map((thread) => (
          <button
            key={thread.id}
            type="button"
            onClick={() => onOpenThread?.(thread)}
            aria-label={thread.purpose}
            title={describeThread(thread)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "6px 9px",
              borderRadius: "var(--radius-sm)",
              border: 0,
              background: "transparent",
              color: "#3a362f",
              fontSize: 12.5,
              textAlign: "left",
              cursor: "pointer",
              width: "100%",
            }}
          >
            <StatusGlyph badge="finished" detail={describeThread(thread)} />
            <span
              style={{
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {thread.purpose}
            </span>
          </button>
        ))}
      </div>

      {navigator ? (
        <>
          <NavRow
            icon="folder"
            label={t("nav.documents")}
            on={false}
            onClick={() => onNavigate("repository")}
            soft
          />
          <SectionLabel>{navigator.label}</SectionLabel>
          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            {navigator.items.map((item) => (
              <div
                key={item.label}
                tabIndex={0}
                style={{
                  fontSize: 12,
                  color: item.on ? "#26241f" : "#5f5a53",
                  fontWeight: item.on ? 600 : 400,
                  background: item.on ? "var(--sel)" : "transparent",
                  padding: item.badge ? "6px 8px" : "5px 8px",
                  paddingLeft: item.indent ? 22 : 8,
                  borderRadius: 6,
                  cursor: "pointer",
                  display: "flex",
                  gap: 8,
                  border: item.badge ? "1px solid var(--border)" : undefined,
                  borderColor: item.badge && item.on ? "#8f8a80" : undefined,
                  marginBottom: item.badge ? 5 : 0,
                }}
              >
                {item.badge ? (
                  <span style={{ fontWeight: 700, color: "#8a8580", fontSize: 9.5 }}>
                    {item.badge}
                  </span>
                ) : null}
                {item.label}
              </div>
            ))}
          </div>
        </>
      ) : (
        <>
          <SectionLabel>{t("nav.documents")}</SectionLabel>
          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            {DOC_ENTRIES.map((entry) => (
              <NavRow
                key={entry.route}
                icon={entry.icon}
                label={entry.label}
                on={route === entry.route}
                onClick={() => onNavigate(entry.route)}
                soft
              />
            ))}
          </div>
        </>
      )}

      <div style={{ marginTop: "auto", paddingTop: 18 }}>
        <NavRow
          icon="settings"
          label={t("nav.settings")}
          on={route === "settings" || route === "privacy" || route === "agents"}
          onClick={() => onNavigate("settings")}
        />
      </div>
    </nav>
  );
}

function NavRow({
  icon,
  label,
  on,
  onClick,
  badge,
  soft = false,
}: {
  icon: Parameters<typeof Icon>[0]["name"];
  label: string;
  on: boolean;
  onClick: () => void;
  badge?: number;
  soft?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={on ? "page" : undefined}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 9,
        padding: "8px 9px",
        borderRadius: "var(--radius-sm)",
        border: 0,
        background: on ? "var(--sel)" : "transparent",
        color: soft && !on ? "#5f5a53" : "#3a362f",
        fontWeight: on ? 600 : 400,
        fontSize: 13.5,
        textAlign: "left",
        cursor: "pointer",
        width: "100%",
      }}
    >
      <Icon name={icon} size={soft ? 13 : 15} stroke={soft ? "#8a8580" : "currentColor"} width={soft ? 1.8 : 2} />
      {label}
      {badge ? (
        <span
          className="ml"
          style={{
            background: "#e4e0d8",
            color: "#5c574f",
            fontSize: 11,
            fontWeight: 650,
            padding: "1px 6px",
            borderRadius: 9,
          }}
        >
          {badge}
        </span>
      ) : null}
    </button>
  );
}
