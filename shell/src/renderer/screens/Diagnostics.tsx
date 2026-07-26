/**
 * Technical details.
 *
 * The only surface in the product exempt from the vocabulary rule (Requirement 1.6,
 * 17.5). It exists so that everywhere else can stay in the User's language: when
 * something goes wrong, the detail a supporter needs has somewhere to live that is not
 * the Dashboard.
 *
 * It is reachable only from Settings and is referenced from nowhere else. Two habits
 * are kept deliberately:
 *
 * - **A figure we do not have is shown as missing, never as zero.** Borrowed from the
 *   adk-rust console's `TelemetryGap`, where the rule is that a legacy token count of
 *   0 must be reported unavailable, because zero is a claim.
 * - **Nothing here leaves the machine unless the User copies it.**
 */

import { t } from "../../shared/strings.ts";
import { useActivity } from "../useOwn.ts";
import { AGENTS } from "../agentFixtures.ts";
import { Button, Icon } from "../components/primitives.tsx";

interface Entry {
  when: string;
  what: string;
  detail: string;
  outcome: "ok" | "recovered" | "failed";
}

const ACTIVITY: Entry[] = [
  {
    when: "09:02:11",
    what: "post_to_x",
    detail: "x-mcp · stdio · 1 call · 412 tokens · 1.8s",
    outcome: "ok",
  },
  {
    when: "07:00:04",
    what: "send_email",
    detail: "mcp-email · gmail · 1 call · 2,204 tokens · 3.1s",
    outcome: "ok",
  },
  {
    when: "06:59:58",
    what: "openai/gpt-5 → anthropic/claude-sonnet",
    detail: "primary returned 429, failover served the request · 6.4s total",
    outcome: "recovered",
  },
  {
    when: "06:40:22",
    what: "set_cell ×5, add_chart",
    detail: "worksheet-mcp · stdio · 6 calls · 5,102 tokens · 4.2s",
    outcome: "ok",
  },
  {
    when: "Thu 18:12",
    what: "list_inbox",
    detail: "mcp-email · gmail · OAuth token expired (invalid_grant)",
    outcome: "failed",
  },
];

const OUTCOME: Record<Entry["outcome"], { label: string; colour: string }> = {
  ok: { label: "ok", colour: "var(--live-fg)" },
  recovered: { label: "recovered", colour: "var(--info-fg)" },
  failed: { label: "failed", colour: "var(--warn-fg)" },
};

const BUILD = [
  ["Work Studio", "0.1.0 (dev)"],
  ["Core", "studio-core 0.1.0 · sqlite 3.50 · WAL"],
  ["Runtime", "adk-rust 2.0.0"],
  ["Spreadsheets", "zavora-xlsx 0.1.1"],
  ["Documents", "zavora-docx 0.1.3"],
  ["Presentations", "zavora-slide 0.1.0"],
  ["Store", "encrypted at rest · 12 tables · migration 0001_init"],
];

export function Diagnostics() {
  const entries = useActivity();
  return (
    <main className="main">
      <h1 className="h1">{t("diag.title")}</h1>
      <p className="hint" style={{ margin: "-6px 0 18px" }}>
        {t("diag.intro")}
      </p>

      <Section label={t("diag.versions")}>
        <table style={{ borderCollapse: "collapse", fontSize: 12.5 }}>
          <tbody>
            {BUILD.map(([name, value]) => (
              <tr key={name}>
                <td style={{ padding: "4px 22px 4px 0", color: "var(--muted)" }}>{name}</td>
                <td
                  style={{
                    padding: "4px 0",
                    fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                    fontSize: 11.5,
                    color: "#3d3933",
                  }}
                >
                  {value}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Section>

      <Section label={t("diag.recent")}>
        {/* The real record, newest first. It was a fixture list while an append-only
            activity_log sat unused. This surface may hold technical detail — it is the one
            place the vocabulary rule exempts, because support needs the cause. */}
        {entries.length === 0 ? (
          <p style={{ fontSize: 12, color: "var(--muted)", margin: "6px 0" }}>
            {t("diag.nothing_yet")}
          </p>
        ) : null}
        {entries.map((entry) => (
          <div
            key={entry.seq}
            style={{
              display: "flex",
              gap: 12,
              alignItems: "flex-start",
              padding: "8px 0",
              borderBottom: "1px solid #f4f2ee",
              fontSize: 12.5,
            }}
          >
            <span
              style={{
                fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                fontSize: 11.5,
                color: "var(--faint)",
                minWidth: 62,
              }}
            >
              {new Date(entry.when * 1000).toLocaleTimeString([], {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div
                style={{
                  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                  fontSize: 11.5,
                  color: "#3d3933",
                }}
              >
                {entry.category}
              </div>
              <div style={{ fontSize: 11.5, color: "var(--faint)", marginTop: 2 }}>
                {entry.detail}
              </div>
            </div>
          </div>
        ))}
      </Section>

      <Section label={t("diag.gaps")}>
        {AGENTS[0]!.gaps.map((gap) => (
          <div
            key={gap.what}
            style={{
              display: "flex",
              gap: 10,
              alignItems: "flex-start",
              padding: "8px 0",
              borderBottom: "1px solid #f4f2ee",
              fontSize: 12.5,
            }}
          >
            <Icon name="info" size={13} stroke="var(--info-fg)" />
            <div style={{ flex: 1 }}>
              <div style={{ color: "#3d3933" }}>{gap.what}</div>
              <div style={{ fontSize: 11.5, color: "var(--faint)", marginTop: 2 }}>{gap.why}</div>
            </div>
            <span style={{ fontSize: 11.5, color: "var(--faint)" }}>{gap.owner}</span>
          </div>
        ))}
        <p className="hint" style={{ marginTop: 8 }}>
          A figure we do not have is shown as missing rather than as zero.
        </p>
      </Section>

      <div style={{ marginTop: 18 }}>
        <Button>{t("diag.copy")}</Button>
      </div>
    </main>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <section style={{ marginBottom: 22 }}>
      <div
        style={{
          fontSize: 10.5,
          fontWeight: 700,
          letterSpacing: ".07em",
          textTransform: "uppercase",
          color: "var(--faint)",
          marginBottom: 10,
        }}
      >
        {label}
      </div>
      {children}
    </section>
  );
}
