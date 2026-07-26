/**
 * The remaining Settings panes.
 *
 * These three existed as tabs before they existed as panes, so selecting them
 * silently showed General — the kind of defect that looks like nothing is wrong.
 *
 * Each answers one question the User might have come here with: which accounts can
 * this act through, where are my files, and what is this costing me.
 */

import { t } from "../../shared/strings.ts";
import { AGENTS } from "../agentFixtures.ts";
import { Button, Icon, Toggle } from "../components/primitives.tsx";

interface Account {
  id: string;
  name: string;
  allowed: string[];
  usedBy: string[];
  status: "connected" | "expired";
}

const ACCOUNTS: Account[] = [
  {
    id: "gmail",
    name: "Gmail · james@zavora.ai",
    allowed: ["Read your inbox", "Send what you approve", "Archive and label"],
    usedBy: ["Inbox triage", "Morning digest", "Daily newsletter"],
    status: "expired",
  },
  {
    id: "calendar",
    name: "Google Calendar",
    allowed: ["Read events", "Create events"],
    usedBy: ["Meeting prep", "Morning digest"],
    status: "connected",
  },
  {
    id: "x",
    name: "X · @zavora_ai",
    allowed: ["Post", "Delete a post it made"],
    usedBy: ["Social posting"],
    status: "connected",
  },
];

export function Accounts() {
  return (
    <div>
      <p className="hint" style={{ margin: "-4px 0 16px" }}>
        {t("accounts.intro")}
      </p>

      {ACCOUNTS.map((account) => (
        <div
          key={account.id}
          style={{
            background: "var(--card)",
            border: "1px solid var(--border)",
            borderRadius: "var(--radius)",
            padding: "14px 16px",
            marginBottom: 9,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div className="title">{account.name}</div>
            {account.status === "expired" ? (
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 5,
                  fontSize: 11,
                  fontWeight: 650,
                  color: "var(--warn-fg)",
                  background: "var(--warn-bg)",
                  padding: "2px 8px",
                  borderRadius: 20,
                }}
              >
                <Icon name="warning" size={11} width={2.4} />
                {t("accounts.expired")}
              </span>
            ) : null}
            <div className="ml" style={{ display: "flex", gap: 7 }}>
              {account.status === "expired" ? (
                <Button small primary>
                  {t("accounts.reconnect")}
                </Button>
              ) : null}
              <Button small>{t("settings.disconnect")}</Button>
            </div>
          </div>

          <div style={{ display: "flex", gap: 28, marginTop: 12, flexWrap: "wrap" }}>
            <div>
              <div style={{ fontSize: 11, color: "var(--faint)", marginBottom: 4 }}>
                It may
              </div>
              {account.allowed.map((line) => (
                <div
                  key={line}
                  style={{ display: "flex", gap: 7, fontSize: 12.5, marginBottom: 3 }}
                >
                  <Icon name="check" size={12} stroke="var(--live-fg)" width={2.4} />
                  {line}
                </div>
              ))}
            </div>
            <div>
              <div style={{ fontSize: 11, color: "var(--faint)", marginBottom: 4 }}>
                {t("accounts.used_by")}
              </div>
              {account.usedBy.map((job) => (
                <div key={job} style={{ fontSize: 12.5, marginBottom: 3, color: "#3d3933" }}>
                  {job}
                </div>
              ))}
            </div>
          </div>
        </div>
      ))}

      <Button style={{ marginTop: 6 }}>{t("accounts.add")}</Button>
    </div>
  );
}

const JOB_FOLDERS = [
  { job: "Expense capture", folder: "Documents › Work Studio › Expenses", files: 6 },
  { job: "Weekly report roll-up", folder: "Documents › Work Studio › Reports", files: 4 },
  { job: "Daily newsletter", folder: "Nothing kept — it goes to your inbox", files: 0 },
];

export function Files() {
  return (
    <div>
      <p className="hint" style={{ margin: "-4px 0 16px" }}>
        {t("files.intro")}
      </p>

      <Row label={t("files.where")}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
          <span style={{ fontSize: 12.5 }}>Documents › Work Studio</span>
          <Button small>{t("settings.change")}</Button>
          <Button small>{t("files.reveal")}</Button>
          <span style={{ fontSize: 12, color: "var(--faint)" }}>
            {t("files.usage")} 4.2 MB · 14 files
          </span>
        </div>
      </Row>

      <Row label={t("files.keep_versions")} hint={t("files.keep_versions_hint")}>
        <Toggle on label={t("files.keep_versions")} />
      </Row>

      <div className="h2" style={{ marginTop: 20 }}>
        {t("files.per_job")}
      </div>
      <p className="hint" style={{ marginBottom: 10 }}>
        {t("files.per_job_hint")}
      </p>
      {JOB_FOLDERS.map((entry) => (
        <div
          key={entry.job}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            padding: "11px 0",
            borderBottom: "1px solid #f2f0eb",
          }}
        >
          <div style={{ minWidth: 180 }}>
            <div className="title" style={{ fontWeight: 560 }}>
              {entry.job}
            </div>
          </div>
          <div style={{ fontSize: 12.5, color: "#3d3933" }}>{entry.folder}</div>
          <span className="ml" style={{ fontSize: 12, color: "var(--faint)" }}>
            {entry.files > 0 ? `${entry.files} files` : "—"}
          </span>
          <Button small>{t("settings.change")}</Button>
        </div>
      ))}
    </div>
  );
}

const BY_JOB = [
  { name: "Daily newsletter", today: "$0.04", month: "$0.86" },
  { name: "Inbox triage", today: "$0.19", month: "$4.02" },
  { name: "Board deck — July", today: "$0.22", month: "$0.22" },
  { name: "Q3 revenue model", today: "$0.14", month: "$1.44" },
  { name: "Computer health", today: "$0.03", month: "$0.71" },
];

export function Spending() {
  return (
    <div>
      <p className="hint" style={{ margin: "-4px 0 16px" }}>
        {t("spend.intro")}
      </p>

      <div style={{ display: "flex", gap: 34, marginBottom: 20, flexWrap: "wrap" }}>
        <Big label={t("spend.today")} value="$0.62" />
        <Big label={t("spend.this_month")} value="$7.25" />
        <Big label={t("spend.limit")} value="$5.00 a day" />
      </div>
      <p className="hint" style={{ marginBottom: 20 }}>
        {t("spend.paused_note")}
      </p>

      <div className="h2">{t("spend.by_work")}</div>
      {BY_JOB.map((row) => (
        <div
          key={row.name}
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 80px 80px",
            padding: "9px 0",
            borderBottom: "1px solid #f2f0eb",
            fontSize: 12.5,
          }}
        >
          <div style={{ color: "#3d3933" }}>{row.name}</div>
          <div style={{ textAlign: "right", color: "var(--muted)" }}>{row.today}</div>
          <div style={{ textAlign: "right", color: "var(--muted)" }}>{row.month}</div>
        </div>
      ))}

      <div className="h2" style={{ marginTop: 22 }}>
        {t("spend.by_agent")}
      </div>
      {AGENTS.map((agent) => (
        <div
          key={agent.id}
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 80px",
            padding: "9px 0",
            borderBottom: "1px solid #f2f0eb",
            fontSize: 12.5,
          }}
        >
          <div style={{ color: "#3d3933" }}>{agent.name}</div>
          <div style={{ textAlign: "right", color: "var(--muted)" }}>{agent.costPerDay}</div>
        </div>
      ))}
    </div>
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

function Big({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div style={{ fontSize: 12, color: "var(--muted)" }}>{label}</div>
      <div style={{ fontSize: 24, fontWeight: 640, letterSpacing: "-.02em", marginTop: 2 }}>
        {value}
      </div>
    </div>
  );
}
