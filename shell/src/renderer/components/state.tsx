/**
 * State indicators.
 *
 * Requirement 21.4 forbids conveying state by colour alone, so each state pairs a
 * colour role with a distinct glyph shape. Requirement 3.8 additionally requires the
 * concrete meaning to be available on hover, on keyboard focus, and as the item's
 * accessible name — the same string in all three places, and a useful fact rather
 * than the state's name: "Next tomorrow, 7:00 am" beats "Scheduled".
 */

import type { ReactNode } from "react";

import { t } from "../../shared/strings.ts";
import type { StateBadge, TrayClass } from "../fixtures.ts";
import { Icon, type IconName } from "./primitives.tsx";

const BADGES: Record<
  StateBadge,
  { icon: IconName | "dot"; colour: string; labelKey: Parameters<typeof t>[0] }
> = {
  working: { icon: "dot", colour: "var(--working)", labelKey: "status.working" },
  scheduled: { icon: "clock", colour: "#8a8580", labelKey: "status.scheduled" },
  needsYou: { icon: "warning", colour: "#b0821f", labelKey: "status.needs_you" },
  finished: { icon: "check", colour: "#4d7a3f", labelKey: "status.done" },
  paused: { icon: "pause", colour: "#8a8580", labelKey: "status.paused" },
};

/** The small indicator beside a thread in the left panel. */
export function StatusGlyph({ badge, detail }: { badge: StateBadge; detail: string }) {
  const spec = BADGES[badge];
  if (spec.icon === "dot") {
    return (
      <span
        role="img"
        aria-label={detail}
        title={detail}
        style={{
          width: 11,
          height: 11,
          flex: "0 0 auto",
          borderRadius: "50%",
          background: spec.colour,
          boxShadow: "0 0 0 2.5px var(--working-halo)",
        }}
      />
    );
  }
  return (
    <span
      role="img"
      aria-label={detail}
      title={detail}
      style={{ display: "grid", placeItems: "center", width: 11, height: 11, flex: "0 0 auto" }}
    >
      <Icon name={spec.icon} size={badge === "needsYou" ? 12 : 11} stroke={spec.colour} width={2.4} />
    </span>
  );
}

/** The plain-language name of a state, for anywhere it must be read as text. */
export function badgeLabel(badge: StateBadge): string {
  return t(BADGES[badge].labelKey);
}

/* --------------------------------------------------------------- tray items */

const CLASSES: Record<
  TrayClass,
  { icon: IconName; edge: string; bg: string; fg: string; labelKey: Parameters<typeof t>[0] }
> = {
  kickoff: {
    icon: "document",
    edge: "var(--review-edge)",
    bg: "#eeecE7",
    fg: "#4a453e",
    labelKey: "tray.kickoff.label",
  },
  escalation: {
    icon: "question",
    edge: "var(--ask-edge)",
    bg: "var(--ask-bg)",
    fg: "var(--ask-fg)",
    labelKey: "tray.escalation.label",
  },
  finding: {
    icon: "info",
    edge: "var(--info-edge)",
    bg: "var(--info-bg)",
    fg: "var(--info-fg)",
    labelKey: "tray.finding.label",
  },
  attention: {
    icon: "warning",
    edge: "var(--warn-edge)",
    bg: "var(--warn-bg)",
    fg: "var(--warn-fg)",
    labelKey: "tray.attention.label",
  },
};

export function trayEdge(cls: TrayClass): string {
  return CLASSES[cls].edge;
}

export function trayLabel(cls: TrayClass): string {
  return t(CLASSES[cls].labelKey);
}

/** The square icon tile that distinguishes one tray class from another. */
export function TrayIcon({ cls }: { cls: TrayClass }) {
  const spec = CLASSES[cls];
  return (
    <span
      aria-hidden="true"
      style={{
        width: 26,
        height: 26,
        borderRadius: 7,
        background: spec.bg,
        color: spec.fg,
        display: "grid",
        placeItems: "center",
        flex: "0 0 auto",
      }}
    >
      <Icon name={spec.icon} size={cls === "attention" ? 15 : 14} />
    </span>
  );
}

export function TrayKind({ cls }: { cls: TrayClass }) {
  const spec = CLASSES[cls];
  return (
    <div
      style={{
        fontSize: 10.5,
        fontWeight: 700,
        letterSpacing: ".07em",
        textTransform: "uppercase",
        color: cls === "kickoff" ? "#5c5750" : spec.fg,
        marginBottom: 3,
      }}
    >
      {trayLabel(cls)}
    </div>
  );
}

/** A row in Waiting on you. */
export function TrayRow({
  cls,
  headline,
  detail,
  actions,
}: {
  cls: TrayClass;
  headline: string;
  detail: string;
  actions: ReactNode;
}) {
  return (
    <div
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        borderLeft: `3px solid ${trayEdge(cls)}`,
        borderRadius: "var(--radius)",
        padding: "12px 14px",
      }}
    >
      <div className="row">
        <TrayIcon cls={cls} />
        <div style={{ minWidth: 0 }}>
          <TrayKind cls={cls} />
          <div className="title">{headline}</div>
          <div className="sub">{detail}</div>
        </div>
        <div className="ml" style={{ display: "flex", gap: 7 }}>
          {actions}
        </div>
      </div>
    </div>
  );
}
