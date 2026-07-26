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
import { THREADS } from "../fixtures.ts";
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
export interface Navigator {
  label: string;
  items: { label: string; on?: boolean; indent?: boolean; badge?: string }[];
}

export function LeftPanel({
  route,
  onNavigate,
  collapsed,
  onToggle,
  waitingCount,
  navigator,
}: {
  route: Route;
  onNavigate: (route: Route, threadId?: string) => void;
  collapsed: boolean;
  onToggle: () => void;
  waitingCount: number;
  navigator?: Navigator;
}) {
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
          paddingTop: 12,
          gap: 14,
          flex: "0 0 auto",
        }}
      >
        <button
          type="button"
          onClick={onToggle}
          title={t("nav.expand")}
          aria-label={t("nav.expand")}
          style={{ border: 0, background: "none", cursor: "pointer", color: "#8a8580" }}
        >
          <Icon name="chevronRight" size={16} />
        </button>
        {THREADS.slice(0, 4).map((thread) => (
          <StatusGlyph key={thread.id} badge={thread.badge} detail={thread.statusDetail} />
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
        padding: "20px 12px",
        display: "flex",
        flexDirection: "column",
        flex: "0 0 auto",
        overflowY: "auto",
      }}
    >
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
        {THREADS.map((thread) => (
          <button
            key={thread.id}
            type="button"
            onClick={() => onNavigate("thread", thread.id)}
            aria-label={`${thread.purpose}, ${thread.statusDetail}`}
            title={thread.statusDetail}
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
            <StatusGlyph badge={thread.badge} detail={thread.statusDetail} />
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
