/**
 * Getting started.
 *
 * First run asks for exactly one thing and helps get it. New work is the single way
 * in, whether the User wants a file made or a piece of work handed over. The library
 * removes the empty state: on first launch there is something to pick, not something
 * to configure.
 */

import { t } from "../../shared/strings.ts";
import { FILES, MORE_TEMPLATES, TEMPLATES } from "../fixtures.ts";
import { Button, Card, Field, Icon } from "../components/primitives.tsx";
import type { Route } from "../routes.ts";

export function FirstRun({ onNavigate }: { onNavigate: (route: Route) => void }) {
  return (
    <div style={{ flex: 1, display: "grid", placeItems: "center", padding: 40 }}>
      <div
        style={{
          width: 452,
          background: "var(--card)",
          border: "1px solid var(--border)",
          borderRadius: 14,
          padding: "26px 28px",
        }}
      >
        <h1 style={{ fontSize: 19, fontWeight: 650, letterSpacing: "-.015em", margin: 0 }}>
          {t("firstrun.welcome")}
        </h1>
        <p className="hint" style={{ margin: "8px 0 20px" }}>
          {t("firstrun.privacy")}
        </p>
        <div style={{ fontSize: 12.5, fontWeight: 600, marginBottom: 2 }}>
          {t("firstrun.key_label")}
        </div>
        <Field placeholder="sk-··········································" />
        <p className="hint" style={{ margin: "4px 0 18px" }}>
          {t("firstrun.key_hint")}
        </p>
        <Button primary style={{ width: "100%", padding: 9 }} onClick={() => onNavigate("library")}>
          {t("firstrun.start")}
        </Button>
        <div
          style={{
            display: "flex",
            justifyContent: "center",
            gap: 14,
            marginTop: 12,
          }}
        >
          <button type="button" style={linkStyle}>
            {t("firstrun.get_a_key")}
          </button>
          <button type="button" style={linkStyle}>
            {t("firstrun.other_provider")}
          </button>
        </div>
      </div>
    </div>
  );
}

const linkStyle = {
  border: 0,
  background: "none",
  color: "var(--muted)",
  fontSize: 12,
  textDecoration: "underline",
  textDecorationColor: "#d5d0c7",
  textUnderlineOffset: "2px",
  cursor: "pointer",
} as const;

export function NewWork({
  onNavigate,
  onOpenFile,
}: {
  onNavigate: (route: Route) => void;
  onOpenFile?: () => void;
}) {
  const recent = FILES.filter((file) => file.kind !== "folder").slice(0, 3);

  return (
    <main className="main">
      <h1 className="h1">{t("new.title")}</h1>
      <Field placeholder={t("new.placeholder")} style={{ padding: "13px 14px", fontSize: 13.5 }} />
      <p className="hint" style={{ margin: "8px 0 12px" }}>
        {t("new.drop")}
      </p>
      {onOpenFile && (
        <div style={{ marginBottom: 20 }}>
          <Button small onClick={onOpenFile}>
            {t("new.open_a_file")}
          </Button>
        </div>
      )}

      <div className="h2">{t("new.recurring")}</div>
      <div className="grid3" style={{ marginBottom: 20 }}>
        {TEMPLATES.slice(0, 2).map((template) => (
          <Card key={template.id}>
            <div className="title">{template.name}</div>
            <div className="sub">{template.what}</div>
          </Card>
        ))}
        <Card>
          <div className="title">{t("new.see_all")}</div>
          <div className="sub">Monitors, digests, meeting prep, expenses.</div>
          <div style={{ marginTop: 9 }}>
            <Button small onClick={() => onNavigate("library")}>
              {t("new.try_it")}
            </Button>
          </div>
        </Card>
      </div>

      <div className="h2">{t("new.resume")}</div>
      <div className="stack">
        {recent.map((file) => (
          <button
            key={file.id}
            type="button"
            onClick={() =>
              onNavigate(
                file.kind === "sheet"
                  ? "spreadsheetWorkspace"
                  : file.kind === "deck"
                    ? "deckWorkspace"
                    : "documentWorkspace",
              )
            }
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              padding: "11px 13px",
              background: "var(--card)",
              border: "1px solid var(--border)",
              borderRadius: 10,
              textAlign: "left",
              cursor: "pointer",
              font: "inherit",
            }}
          >
            <Icon
              name={file.kind === "sheet" ? "sheet" : file.kind === "deck" ? "deck" : "document"}
              size={17}
              stroke="var(--muted)"
              width={1.8}
            />
            <div>
              <div className="title" style={{ fontWeight: 560 }}>
                {file.name}
              </div>
              <div className="sub">{file.sub}</div>
            </div>
            <span className="ml" style={{ fontSize: 12, color: "var(--faint)" }}>
              {file.versions} versions
            </span>
          </button>
        ))}
      </div>
      <p className="hint" style={{ marginTop: 14 }}>
        {t("new.files_live")}
      </p>
    </main>
  );
}

export function RecurringLibrary({ onNavigate }: { onNavigate: (route: Route) => void }) {
  return (
    <main className="main">
      <h1 className="h1">{t("library.title")}</h1>
      <p className="hint" style={{ margin: "-6px 0 16px" }}>
        {t("library.sub")}
      </p>

      <div className="grid3" style={{ gap: 11 }}>
        {TEMPLATES.map((template) => (
          <Card key={template.id}>
            <div className="title">{template.name}</div>
            <div className="sub">{template.what}</div>
            <div style={{ display: "flex", alignItems: "center", gap: 9, marginTop: 14 }}>
              <Button
                small
                onClick={() =>
                  onNavigate(template.id === "tpl-triage" ? "kickoffManifest" : "kickoffOutput")
                }
              >
                {t("new.try_it")}
              </Button>
              {template.needs ? (
                <span style={{ fontSize: 12, color: "var(--faint)" }}>Needs {template.needs}</span>
              ) : (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    fontSize: 12,
                    color: "var(--live-fg)",
                    fontWeight: 600,
                  }}
                >
                  <Icon name="check" size={12} width={2.4} />
                  {t("new.ready")}
                </span>
              )}
            </div>
          </Card>
        ))}
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 18,
          marginTop: 20,
          alignItems: "start",
        }}
      >
        <section aria-label={t("library.more")}>
          <div className="h2">{t("library.more")}</div>
          <div className="stack">
            {MORE_TEMPLATES.map((template) => (
              <Card key={template.id} style={{ padding: "11px 14px" }}>
                <div className="row">
                  <div className="title" style={{ fontWeight: 560 }}>
                    {template.name}
                  </div>
                  <span className="ml" style={{ fontSize: 12, color: "var(--faint)" }}>
                    {template.needs ?? t("new.ready")}
                  </span>
                </div>
              </Card>
            ))}
          </div>
        </section>

        <ConsentPanel onNavigate={onNavigate} />
      </div>
    </main>
  );
}

function ConsentPanel({ onNavigate }: { onNavigate: (route: Route) => void }) {
  return (
    <div
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        borderRadius: 14,
        padding: "20px 22px",
      }}
    >
      <div style={{ fontSize: 14.5, fontWeight: 650 }}>{t("consent.title")}</div>
      <p className="hint" style={{ margin: "8px 0 14px" }}>
        {t("consent.intro")}
      </p>
      <div style={{ fontSize: 12.5, lineHeight: 1.9, marginBottom: 14 }}>
        {[t("consent.read"), t("consent.label"), t("consent.draft")].map((line) => (
          <div key={line} style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
            <Icon name="check" size={13} stroke="var(--live-fg)" width={2.4} />
            <span>{line}</span>
          </div>
        ))}
      </div>
      <p className="hint" style={{ marginBottom: 16 }}>
        {t("consent.never_send")}
      </p>
      <div style={{ display: "flex", gap: 9 }}>
        <Button primary onClick={() => onNavigate("kickoffManifest")}>
          {t("consent.connect")}
        </Button>
        <Button>{t("consent.not_now")}</Button>
      </div>
    </div>
  );
}
