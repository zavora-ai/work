/**
 * The first draft, in its two shapes.
 *
 * A first-draft review is not a permission dialog. It shows the work the User would
 * have received, and the buttons are about the work rather than about consent
 * (Requirement 5.6).
 *
 * Two shapes, because most pieces of work do not produce a document:
 *
 * * **output** — a newsletter, a message, a file. The full thing, as it would have
 *   arrived.
 * * **actions** — a manifest of what would have been done, each row with its count
 *   and whether it can be undone, each row individually excludable (Requirement 5.8).
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { Button, Card, Icon } from "../components/primitives.tsx";
import { TrayIcon } from "../components/state.tsx";

export function KickoffOutput() {
  return (
    <main className="main">
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
        <TrayIcon cls="kickoff" />
        <h1 className="h1" style={{ margin: 0 }}>
          {t("kickoff.output.title")}
        </h1>
      </div>
      <p className="hint" style={{ margin: "0 0 14px 36px" }}>
        {t("kickoff.output.sub")}
      </p>

      <div
        style={{
          background: "var(--card)",
          border: "1px solid var(--border)",
          borderRadius: 10,
          padding: "20px 24px",
          fontSize: 12.5,
          lineHeight: 1.65,
          color: "#2b2823",
        }}
      >
        <h2 style={{ fontSize: 15, margin: "0 0 3px" }}>Your Monday brief</h2>
        <div style={{ fontSize: 11.5, color: "var(--faint)", marginBottom: 12 }}>
          3 sources · 6 minute read
        </div>
        <div style={{ fontWeight: 600, marginBottom: 6 }}>What moved</div>
        <ul style={{ margin: "0 0 12px", paddingLeft: 18 }}>
          <li style={{ marginBottom: 7 }}>
            The EU AI Act's general-purpose obligations took effect Friday. Vendors with EU
            customers now need documentation on file.
          </li>
          <li style={{ marginBottom: 7 }}>
            Two of your three tracked competitors shipped pricing changes this week, both moving
            to usage-based tiers.
          </li>
          <li style={{ marginBottom: 7 }}>
            Kenyan T-bill yields eased for a third week, now averaging 11.4% on the 91-day.
          </li>
        </ul>
        <div style={{ fontWeight: 600, marginBottom: 6 }}>Worth a look</div>
        <ul style={{ margin: 0, paddingLeft: 18 }}>
          <li style={{ marginBottom: 7 }}>
            A long read on why on-device inference is getting cheaper faster than cloud inference.
          </li>
          <li>Your saved search "agent framework" turned up four new repositories over 1k stars.</li>
        </ul>
      </div>

      <div style={{ display: "flex", gap: 9, marginTop: 14 }}>
        <Button primary>{t("kickoff.approve_daily")}</Button>
        <Button>{t("kickoff.edit_first")}</Button>
        <Button>{t("kickoff.reject")}</Button>
      </div>
    </main>
  );
}

interface IntendedAction {
  verb: string;
  description: string;
  reversal: string;
}

const MANIFEST: IntendedAction[] = [
  {
    verb: "Archive",
    description: "18 newsletters and receipts you've never opened",
    reversal: t("kickoff.manifest.reversible"),
  },
  {
    verb: "Label",
    description: "9 messages as Needs reply — 3 of them are from clients",
    reversal: t("kickoff.manifest.reversible"),
  },
  {
    verb: "Draft",
    description: "4 replies, left unsent in your drafts folder",
    reversal: t("kickoff.manifest.you_send"),
  },
  { verb: "Leave alone", description: "11 messages I wasn't confident about", reversal: "—" },
];

export function KickoffManifest() {
  const [excluded, setExcluded] = useState<Set<number>>(new Set());

  const toggle = (index: number) => {
    setExcluded((current) => {
      const next = new Set(current);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  return (
    <main className="main">
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
        <TrayIcon cls="kickoff" />
        <h1 className="h1" style={{ margin: 0 }}>
          {t("kickoff.manifest.title")}
        </h1>
      </div>
      <p className="hint" style={{ margin: "0 0 14px 36px" }}>
        {t("kickoff.manifest.sub")}
      </p>

      <div
        style={{
          border: "1px solid var(--border)",
          borderRadius: 10,
          overflow: "hidden",
          background: "var(--card)",
        }}
      >
        <div
          style={{
            padding: "10px 14px",
            borderBottom: "1px solid var(--border)",
            fontSize: 12,
            fontWeight: 650,
            color: "var(--ink-soft)",
            background: "#fbfaf8",
          }}
        >
          {t("kickoff.manifest.count")}
        </div>
        {MANIFEST.map((row, index) => {
          const off = excluded.has(index);
          return (
            <label
              key={row.verb}
              style={{
                display: "flex",
                gap: 11,
                alignItems: "center",
                padding: "9px 14px",
                borderBottom: index < MANIFEST.length - 1 ? "1px solid #f0eeE9" : 0,
                fontSize: 12.5,
                opacity: off ? 0.45 : 1,
                cursor: "pointer",
              }}
            >
              <input
                type="checkbox"
                checked={!off}
                onChange={() => toggle(index)}
                aria-label={`${row.verb}: ${row.description}`}
              />
              <span
                style={{ fontWeight: 600, minWidth: 78, fontSize: 11.5, color: "var(--ink-soft)" }}
              >
                {row.verb}
              </span>
              <span style={{ flex: 1, color: "#3d3933" }}>{row.description}</span>
              <span style={{ fontSize: 11.5, color: "var(--faint)" }}>{row.reversal}</span>
            </label>
          );
        })}
      </div>

      <Card style={{ marginTop: 14 }}>
        <div className="row">
          <div>
            <div className="title">{t("kickoff.read_one")}</div>
            <div className="sub">Reply to Achieng about the Thursday deadline</div>
          </div>
          <Button small style={{ marginLeft: "auto" }}>
            {t("kickoff.open_draft")}
          </Button>
        </div>
      </Card>

      <div style={{ display: "flex", gap: 9, marginTop: 14 }}>
        <Button primary>{t("kickoff.approve_recurring")}</Button>
        <Button>{t("kickoff.once")}</Button>
        <Button>{t("kickoff.reject")}</Button>
      </div>
      <p className="hint" style={{ marginTop: 12, display: "flex", alignItems: "center", gap: 6 }}>
        <Icon name="clock" size={12} stroke="var(--faint)" />
        {t("kickoff.can_undo")}
      </p>
    </main>
  );
}
