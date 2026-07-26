/**
 * Waiting on you, Done for you, and one thread in detail.
 *
 * The four tray classes are separated by glyph shape, label and edge treatment, so
 * that a first draft to check is never confusable with a fault, and a Finding — a
 * piece of work that succeeded and found something — is never dressed as one.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { useDelivered, useSteering, useThreads, useTurns, useWaiting } from "../useOwn.ts";
import { Button, Card, Field, Pill } from "../components/primitives.tsx";
import { Empty } from "../components/states.tsx";
import { TrayRow } from "../components/state.tsx";
import type { Route } from "../routes.ts";

export function InTray({ onNavigate }: { onNavigate: (route: Route) => void }) {
  const { items, decide } = useWaiting();

  return (
    <main className="main">
      <h1 className="h1">{t("tray.heading")}</h1>
      {items.length === 0 ? (
        <Empty
          icon="check"
          title={t("dash.nothing_waiting")}
          what="Anything that needs a decision from you will appear here. Nothing expires while it waits."
        />
      ) : null}
      <div className="stack">
        {/* Real items from the tray. A refusal a specialist ran into is here with the
            actions that resolve it — and answering removes it, because the tray refuses a
            second answer rather than letting one decision be made twice. */}
        {items.map((item) => (
          <TrayRow
            key={item.id}
            cls={item.sort as "kickoff" | "escalation" | "finding" | "attention"}
            headline={item.headline}
            detail={item.detail}
            actions={(item.choices.length > 0 ? item.choices : ["Dismiss"]).map(
              (choice, index) => (
                <Button
                  key={choice}
                  small={item.choices.length > 1}
                  onClick={() => {
                    void decide(item.id, choice === "Dismiss" ? "dismiss" : choice);
                    if (item.sort === "kickoff") onNavigate("kickoffOutput");
                  }}
                  primary={index === 0 && item.choices.length === 1 && item.sort === "attention"}
                >
                  {choice}
                </Button>
              ),
            )}
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
  const [steering, setSteering] = useState<string | undefined>(undefined);
  const delivered = useDelivered();

  return (
    <main className="main">
      <h1 className="h1">{t("out.heading")}</h1>
      {delivered.length === 0 ? (
        <Empty
          icon="bolt"
          title="Nothing has been done yet"
          what="Once something runs on its own, what it did appears here — with a way to undo it where that is possible."
        />
      ) : null}
      <div className="stack">
        {delivered.map((delivery) => (
          <Card key={delivery.id}>
            <div className="row">
              <div style={{ minWidth: 0 }}>
                <div className="title">{delivery.what}</div>
                <div className="sub">
                  {new Date(delivery.when * 1000).toLocaleString([], {
                    day: "numeric",
                    month: "short",
                    hour: "2-digit",
                    minute: "2-digit",
                  })}{" "}
                  · {delivery.whereTo}
                </div>
              </div>
              <div className="ml" style={{ display: "flex", gap: 7, alignItems: "center" }}>
                {/* An undo is offered only where one exists. The schema will not let a
                    delivery claim a window it does not have, so this cannot over-promise. */}
                {delivery.reversed ? (
                  <span style={{ fontSize: 12, color: "var(--faint)" }}>
                    {t("out.already_undone")}
                  </span>
                ) : delivery.reversible ? (
                  <Button small>{t("out.undo")}</Button>
                ) : (
                  <span style={{ fontSize: 12, color: "var(--faint)" }}>
                    {t("out.cannot_undo")}
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
  // A piece of work, as it actually exists: what the User asked for, the conversation that
  // followed, and what they have told Work Studio about it.
  //
  // The schedule, the spend and the Pause and Run now buttons are gone. They belonged to
  // proactive work that does not exist yet, and a button that changes nothing is worse than an
  // absent one — the User presses it and learns not to trust the screen.
  const { threads } = useThreads();
  const thread = threads.find((candidate) => candidate.id === threadId);
  const turns = useTurns(threadId);
  const { state, add } = useSteering(threadId);
  const [typed, setTyped] = useState("");

  if (!thread) {
    return (
      <main className="main">
        <Empty
          icon="check"
          title={t("thread.not_found")}
          what="Open a file and ask for something, and it will be here."
        />
      </main>
    );
  }

  const keep = async () => {
    if (!typed.trim()) return;
    await add(typed);
    setTyped("");
  };

  return (
    <main className="main">
      <div style={{ marginBottom: 16 }}>
        <h1 className="h1" style={{ marginBottom: 3 }}>
          {thread.purpose}
        </h1>
        {thread.file ? <p className="hint">{thread.file}</p> : null}
      </div>

      <div className="cols">
        <section aria-label={t("thread.what_its_done")}>
          <div className="h2">{t("thread.what_its_done")}</div>
          <div className="stack">
            {turns.length === 0 ? (
              <p className="hint">{t("thread.nothing_said_yet")}</p>
            ) : null}
            {turns.map((turn, index) => (
              <Card key={`${index}-${turn.text.slice(0, 12)}`} style={{ padding: "11px 14px" }}>
                <div className="title" style={{ fontWeight: 560 }}>
                  {turn.text}
                </div>
                <div className="sub">{turn.from === "you" ? t("thread.you") : t("thread.me")}</div>
              </Card>
            ))}
          </div>
        </section>

        <section aria-label={t("thread.learned")}>
          <div className="h2">{t("thread.learned")}</div>
          <div className="stack">
            {state.notes.length === 0 ? (
              <p className="hint">{t("thread.nothing_told_yet")}</p>
            ) : null}
            {state.notes.map((note) => (
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
              </div>
            ))}
            <Field
              placeholder={t("thread.new_note")}
              value={typed}
              onChange={(event) => setTyped(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void keep();
              }}
            />
            <p className="hint">{t("thread.everything_i_go_on")}</p>
          </div>
        </section>
      </div>
    </main>
  );
}
