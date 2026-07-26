/**
 * Waiting on you, Done for you, and one thread in detail.
 *
 * The four tray classes are separated by glyph shape, label and edge treatment, so
 * that a first draft to check is never confusable with a fault, and a Finding — a
 * piece of work that succeeded and found something — is never dressed as one.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import {
  DELIVERIES,
  RUN_HISTORY,
  THREADS,
  THREAD_NOTES,
  TRAY,
  type Thread,
} from "../fixtures.ts";
import { Button, Card, Field, Pill } from "../components/primitives.tsx";
import { Empty } from "../components/states.tsx";
import { TrayRow } from "../components/state.tsx";
import type { Route } from "../routes.ts";

export function InTray({ onNavigate }: { onNavigate: (route: Route) => void }) {
  return (
    <main className="main">
      <h1 className="h1">{t("tray.heading")}</h1>
      {TRAY.length === 0 ? (
        <Empty
          icon="check"
          title={t("dash.nothing_waiting")}
          what="Anything that needs a decision from you will appear here. Nothing expires while it waits."
        />
      ) : null}
      <div className="stack">
        {TRAY.map((item) => (
          <TrayRow
            key={item.id}
            cls={item.cls}
            headline={item.headline}
            detail={item.detail}
            actions={item.choices.map((choice, index) => (
              <Button
                key={choice}
                small={item.choices.length > 1}
                onClick={() =>
                  onNavigate(item.cls === "kickoff" ? "kickoffOutput" : "tray")
                }
                primary={index === 0 && item.choices.length === 1 && item.cls === "attention"}
              >
                {choice}
              </Button>
            ))}
          />
        ))}
      </div>
      <p className="hint" style={{ marginTop: 16 }}>
        {t("tray.nothing_expires")}
      </p>
    </main>
  );
}

export function OutTray() {
  const [steering, setSteering] = useState<string | undefined>("d-1");

  return (
    <main className="main">
      <h1 className="h1">{t("out.heading")}</h1>
      {DELIVERIES.length === 0 ? (
        <Empty
          icon="bolt"
          title="Nothing has been done yet"
          what="Once something runs on its own, what it did appears here — with a way to undo it where that is possible."
        />
      ) : null}
      <div className="stack">
        {DELIVERIES.map((delivery) => (
          <Card key={delivery.id}>
            <div className="row">
              <div style={{ minWidth: 0 }}>
                <div className="title">{delivery.action}</div>
                <div className="sub">
                  {delivery.when} · {delivery.thread}
                  {delivery.extra ? ` · ${delivery.extra}` : ""}
                </div>
              </div>
              <div className="ml" style={{ display: "flex", gap: 7, alignItems: "center" }}>
                {delivery.reversal.kind === "available" ? (
                  <Button small>{delivery.reversal.label}</Button>
                ) : (
                  <span style={{ fontSize: 12, color: "var(--faint)" }}>
                    {delivery.reversal.reason}
                  </span>
                )}
                <Button small onClick={() => setSteering(delivery.id)}>
                  {t("out.steer")}
                </Button>
              </div>
            </div>

            {steering === delivery.id ? (
              <>
                <Field
                  value="Too promotional — write like a person next time."
                  style={{ marginTop: 11 }}
                />
                <p className="hint" style={{ marginTop: 6 }}>
                  {t("out.steer_saved")}
                </p>
              </>
            ) : null}
          </Card>
        ))}
      </div>
    </main>
  );
}

export function ThreadDetail({ threadId }: { threadId?: string }) {
  const thread: Thread = THREADS.find((candidate) => candidate.id === threadId) ?? THREADS[0]!;
  const scheduled = Boolean(thread.scheduleHuman);

  return (
    <main className="main">
      <div style={{ display: "flex", alignItems: "flex-start", gap: 12, marginBottom: 16 }}>
        <div>
          <h1 className="h1" style={{ marginBottom: 3 }}>
            {thread.purpose}
          </h1>
          <p className="hint">
            {[thread.scheduleHuman, thread.nextHuman && `next ${thread.nextHuman.toLowerCase()}`, thread.spendToday]
              .filter(Boolean)
              .join(" · ")}
          </p>
        </div>
        <div className="ml" style={{ display: "flex", gap: 8, alignItems: "center" }}>
          {thread.badge === "needsYou" ? (
            <Pill tone="warn" icon="warning">
              Paused · Gmail
            </Pill>
          ) : (
            <Pill tone="live">Working</Pill>
          )}
          {scheduled ? <Button small>{t("thread.pause")}</Button> : null}
          <Button small>{t("thread.run_now")}</Button>
        </div>
      </div>

      <div className="cols">
        <section aria-label={t("thread.what_its_done")}>
          <div className="h2">{t("thread.what_its_done")}</div>
          <div className="stack">
            {RUN_HISTORY.map((entry) => (
              <Card key={entry.when} style={{ padding: "11px 14px" }}>
                <div className="title" style={{ fontWeight: 560 }}>
                  {entry.text}
                </div>
                <div className="sub">{entry.when}</div>
              </Card>
            ))}
          </div>
        </section>

        <section aria-label={t("thread.learned")}>
          <div className="h2">{t("thread.learned")}</div>
          <div className="stack">
            {THREAD_NOTES.map((note) => (
              <div
                key={note.id}
                style={{
                  background: "var(--card)",
                  border: "1px solid var(--border)",
                  borderRadius: 9,
                  padding: "10px 12px",
                  display: "flex",
                  gap: 10,
                  alignItems: "flex-start",
                }}
              >
                <div style={{ flex: 1, fontSize: 12.5, lineHeight: 1.4 }}>
                  {note.note}
                  <div style={{ fontSize: 11, color: "var(--faint)", marginTop: 3 }}>
                    {note.provenance}
                  </div>
                </div>
                <Button small>{t("thread.edit")}</Button>
              </div>
            ))}
            <Field placeholder={t("thread.new_note")} />
            <p className="hint">{t("thread.everything_i_go_on")}</p>
          </div>
        </section>
      </div>
    </main>
  );
}
