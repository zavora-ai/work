/**
 * Getting started.
 *
 * First run asks for exactly one thing and helps get it. New work is the single way
 * in, whether the User wants a file made or a piece of work handed over. The library
 * removes the empty state: on first launch there is something to pick, not something
 * to configure.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { MORE_TEMPLATES, TEMPLATES } from "../fixtures.ts";
import { bridge, useThreads } from "../useOwn.ts";

/// Which glyph a file gets, from the file itself.
function iconFor(path: string): "sheet" | "deck" | "document" {
  if (path.endsWith(".xlsx")) return "sheet";
  if (path.endsWith(".pptx")) return "deck";
  return "document";
}
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
  onStarted,
}: {
  onNavigate: (route: Route) => void;
  onOpenFile?: () => void;
  /** Called with the file Work Studio made, so it can be opened straight away. */
  onStarted?: (path: string, asked: string) => void;
}) {
  // The front door. It took a sentence and did nothing at all: the User typed what they needed,
  // pressed return, and the screen sat still.
  const { threads } = useThreads();
  const recent = threads.filter((thread) => thread.file).slice(0, 3);
  const [typed, setTyped] = useState("");
  const [starting, setStarting] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();

  const begin = async () => {
    const asked = typed.trim();
    if (!asked || starting) return;
    setStarting(true);
    setRefused(undefined);
    try {
      const answer = (await bridge()?.start?.({ asked })) as
        | { path?: string; problem?: string }
        | undefined;
      if (answer?.path) {
        setTyped("");
        // Handed straight to the specialist: the User asked for a thing, so the thing opens and
        // is filled in while they watch, rather than arriving empty with nothing said.
        onStarted?.(answer.path, asked);
      } else {
        setRefused(answer?.problem ?? t("new.could_not_start"));
      }
    } catch {
      setRefused(t("new.could_not_start"));
    } finally {
      setStarting(false);
    }
  };

  return (
    <main className="main">
      <h1 className="h1">{t("new.title")}</h1>
      <Field
        placeholder={t("new.placeholder")}
        value={typed}
        disabled={starting}
        onChange={(event) => setTyped(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") void begin();
        }}
        style={{ padding: "13px 14px", fontSize: 13.5 }}
      />
      <div style={{ display: "flex", gap: 8, alignItems: "center", margin: "9px 0 12px" }}>
        <Button primary onClick={() => void begin()} disabled={starting || !typed.trim()}>
          {starting ? t("new.starting") : t("new.make_a_start")}
        </Button>
        {refused ? (
          <span style={{ fontSize: 12, color: "var(--warn-ink, #8a5a00)" }}>{refused}</span>
        ) : null}
      </div>
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
        {/* What the User was actually last working on, opened by its own path rather than by
            navigating to a workspace that might be showing something else. */}
        {recent.map((work) => (
          <button
            key={work.id}
            type="button"
            onClick={() => onStarted?.(work.file!, work.purpose)}
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
              name={iconFor(work.file!)}
              size={17}
              stroke="var(--muted)"
              width={1.8}
            />
            <div>
              <div className="title" style={{ fontWeight: 560 }}>
                {work.file!.slice(work.file!.lastIndexOf("/") + 1)}
              </div>
              <div className="sub">{work.purpose}</div>
            </div>
            {/* The fixture said "7 versions" here. Nothing counts versions, so nothing is
                claimed: the time it was last touched is something we actually know. */}
            <span className="ml" style={{ fontSize: 12, color: "var(--faint)" }}>
              {new Date(work.changed * 1000).toLocaleDateString([], {
                day: "numeric",
                month: "short",
              })}
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
