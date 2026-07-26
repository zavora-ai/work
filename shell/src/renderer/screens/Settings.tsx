/**
 * Settings.
 *
 * One screen, six sections, no configuration ritual. This is the only place a
 * provider may be named (Requirement 14.7), and the single link to technical detail
 * sits at the foot and is referenced from nowhere else.
 *
 * `How I should work` is the global twin of the per-thread steering list, because
 * "use our brand colours" belongs to every deck rather than to one. Per-thread notes
 * win, and the screen says so rather than leaving precedence to be discovered.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { GLOBAL_NOTES } from "../fixtures.ts";
import { AgentsSettings } from "./AgentsSettings.tsx";
import { Accounts, Files, Spending } from "./SettingsPanes.tsx";
import { Button, Field, Icon, Segmented, Toggle } from "../components/primitives.tsx";

const TABS = [
  t("settings.tab.general"),
  t("settings.tab.how_i_work"),
  t("settings.tab.agents"),
  t("settings.tab.accounts"),
  t("settings.tab.files"),
  t("settings.tab.spending"),
  t("settings.tab.privacy"),
];

export function Settings({
  initialTab,
  onDiagnostics,
}: {
  initialTab?: string;
  onDiagnostics?: () => void;
}) {
  const [tab, setTab] = useState(initialTab ?? TABS[0]!);

  return (
    <main className="main">
      <h1 className="h1">{t("settings.title")}</h1>
      <div
        style={{ display: "flex", gap: 3, borderBottom: "1px solid var(--border)", marginBottom: 18 }}
      >
        {TABS.map((option) => {
          const on = option === tab;
          return (
            <button
              key={option}
              type="button"
              role="tab"
              aria-selected={on}
              onClick={() => setTab(option)}
              style={{
                border: 0,
                background: "none",
                fontSize: 12.5,
                fontWeight: on ? 650 : 560,
                color: on ? "#26241f" : "#6d6862",
                padding: "7px 11px",
                borderBottom: `2px solid ${on ? "#26241f" : "transparent"}`,
                cursor: "pointer",
              }}
            >
              {option}
            </button>
          );
        })}
      </div>

      {tab === t("settings.tab.how_i_work") ? (
        <HowIShouldWork />
      ) : tab === t("settings.tab.agents") ? (
        <AgentsSettings />
      ) : tab === t("settings.tab.accounts") ? (
        <Accounts />
      ) : tab === t("settings.tab.files") ? (
        <Files />
      ) : tab === t("settings.tab.spending") ? (
        <Spending />
      ) : tab === t("settings.tab.privacy") ? (
        <Privacy />
      ) : (
        <General onDiagnostics={onDiagnostics} />
      )}
    </main>
  );
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 16,
        padding: "14px 0",
        borderBottom: "1px solid #f2f0eb",
      }}
    >
      <div style={{ width: 186, flex: "0 0 auto" }}>
        <div style={{ fontSize: 13, fontWeight: 600 }}>{label}</div>
        {hint ? (
          <div style={{ fontSize: 11.5, color: "var(--faint)", marginTop: 2, lineHeight: 1.4 }}>
            {hint}
          </div>
        ) : null}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>{children}</div>
    </div>
  );
}

function General({ onDiagnostics }: { onDiagnostics?: () => void }) {
  const [tier, setTier] = useState(t("settings.tier.balanced"));

  return (
    <div>
      <Row label={t("settings.ai_key")} hint={t("settings.ai_key_hint")}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span style={{ fontSize: 12.5 }}>
            OpenAI —{" "}
            <span style={{ color: "var(--live-fg)", fontWeight: 600 }}>{t("settings.working")}</span>
          </span>
          <Button small>{t("settings.replace")}</Button>
          <Button small>{t("settings.add_provider")}</Button>
        </div>
      </Row>

      <Row label={t("settings.how_hard")} hint={t("settings.how_hard_hint")}>
        <Segmented
          options={[t("settings.tier.cheap"), t("settings.tier.balanced"), t("settings.tier.best")]}
          active={tier}
          onSelect={setTier}
        />
      </Row>

      <Row label={t("settings.launch")} hint={t("settings.launch_hint")}>
        <Toggle on label={t("settings.launch")} />
      </Row>

      <Row label={t("settings.files_live")} hint={t("settings.files_live_hint")}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span style={{ fontSize: 12.5 }}>Documents › Work Studio</span>
          <Button small>{t("settings.change")}</Button>
        </div>
      </Row>

      <Row label={t("settings.daily_limit")} hint={t("settings.daily_limit_hint")}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <Field value="$5.00" style={{ width: 82, padding: "6px 10px", fontSize: 12.5 }} />
          <span style={{ fontSize: 12, color: "var(--faint)" }}>$0.62 {t("settings.used_today")}</span>
        </div>
      </Row>

      <Row label={t("settings.your_data")} hint={t("settings.your_data_hint")}>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <Button small>{t("settings.what_leaves")}</Button>
          <Button small>{t("settings.export")}</Button>
          <Button small>{t("settings.delete")}</Button>
        </div>
      </Row>

      <p className="hint" style={{ marginTop: 16 }}>
        {t("settings.not_working")}{" "}
        <button
          type="button"
          onClick={onDiagnostics}
          style={{
            border: 0,
            background: "none",
            font: "inherit",
            color: "inherit",
            padding: 0,
            cursor: "pointer",
            textDecoration: "underline",
            textDecorationColor: "#d5d0c7",
            textUnderlineOffset: 2,
          }}
        >
          {t("settings.technical_details")}
        </button>{" "}
        — {t("settings.support_only")}
      </p>
    </div>
  );
}

function HowIShouldWork() {
  const [scope, setScope] = useState(t("settings.scope.everything"));

  return (
    <div>
      <p className="hint" style={{ margin: "-4px 0 14px" }}>
        {t("settings.how_i_work_intro")}
      </p>

      <div className="stack">
        {GLOBAL_NOTES.map((note) => (
          <div
            key={note.id}
            style={{
              background: "var(--card)",
              border: "1px solid var(--border)",
              borderRadius: 10,
              padding: "11px 13px",
              display: "flex",
              gap: 12,
              alignItems: "flex-start",
            }}
          >
            <span
              style={{
                fontSize: 10.5,
                fontWeight: 700,
                letterSpacing: ".05em",
                textTransform: "uppercase",
                color: "var(--ink-soft)",
                background: "#f1efea",
                padding: "2px 7px",
                borderRadius: 5,
                whiteSpace: "nowrap",
              }}
            >
              {note.scope}
            </span>
            <div style={{ flex: 1, fontSize: 12.5, lineHeight: 1.45 }}>
              {note.note}
              <div style={{ fontSize: 11, color: "var(--faint)", marginTop: 3 }}>
                {note.provenance}
              </div>
            </div>
            <Button small>{t("thread.edit")}</Button>
          </div>
        ))}
      </div>

      <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 12 }}>
        <Field placeholder={t("settings.new_note")} style={{ flex: 1 }} />
        <Segmented
          small
          options={[
            t("settings.scope.everything"),
            t("settings.scope.documents"),
            t("settings.scope.decks"),
            t("settings.scope.spreadsheets"),
          ]}
          active={scope}
          onSelect={setScope}
        />
      </div>
      <p className="hint" style={{ marginTop: 14 }}>
        {t("settings.thread_wins")}
      </p>
    </div>
  );
}

const ACCOUNTS = [
  {
    name: "Gmail · james@zavora.ai",
    what: "Reads your inbox, sends what you approve · used by 3 tasks",
  },
  { name: "Google Calendar", what: "Reads events, creates events · used by 2 tasks" },
  { name: "X · @zavora_ai", what: "Posts and deletes posts · used by 1 task" },
];

export function Privacy() {
  return (
    <div>
      <h2 style={{ fontSize: 15, fontWeight: 650, margin: "0 0 4px" }}>{t("privacy.title")}</h2>
      <p className="hint" style={{ margin: "0 0 18px" }}>
        {t("privacy.sub")}
      </p>

      <Flow from={t("privacy.your_words")} to="OpenAI" note={t("privacy.to_write")} />
      <p className="hint" style={{ margin: "9px 0 14px" }}>
        {t("privacy.only_part")}
      </p>
      <Flow from={t("privacy.outputs")} to="Gmail, X" note={t("privacy.because")} />

      <div className="h2" style={{ marginTop: 22 }}>
        {t("privacy.accounts")}
      </div>
      {ACCOUNTS.map((account) => (
        <div
          key={account.name}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            padding: "12px 0",
            borderBottom: "1px solid #f0eeE9",
          }}
        >
          <div>
            <div className="title" style={{ fontWeight: 560 }}>
              {account.name}
            </div>
            <div className="sub">{account.what}</div>
          </div>
          <Button small style={{ marginLeft: "auto" }}>
            {t("settings.disconnect")}
          </Button>
        </div>
      ))}

      <div style={{ display: "flex", gap: 9, marginTop: 20, alignItems: "center" }}>
        <Button>{t("settings.export")}</Button>
        <Button>{t("settings.delete")}</Button>
        <span
          className="ml"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            fontSize: 12,
            color: "var(--live-fg)",
            fontWeight: 600,
          }}
        >
          <Icon name="lock" size={13} />
          {t("privacy.locked")}
        </span>
      </div>
    </div>
  );
}

function Flow({ from, to, note }: { from: string; to: string; note: string }) {
  return (
    <div
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        borderRadius: "var(--radius)",
        padding: 16,
        display: "flex",
        alignItems: "center",
        gap: 8,
        fontSize: 12.5,
      }}
    >
      <span
        style={{
          background: "#fff",
          border: "1px solid var(--border)",
          borderRadius: 8,
          padding: "7px 11px",
        }}
      >
        {from}
      </span>
      <Icon name="arrowRight" size={18} stroke="var(--faint)" width={1.8} />
      <span
        style={{
          background: "#fff",
          border: "1px solid var(--border)",
          borderRadius: 8,
          padding: "7px 11px",
        }}
      >
        <b>{to}</b> — {note}
      </span>
    </div>
  );
}
