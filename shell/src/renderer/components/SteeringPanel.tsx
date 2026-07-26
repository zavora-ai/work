/**
 * What Work Studio goes on.
 *
 * The panel that says "This is everything I go on" — and now does, because the notes come
 * from the store rather than from three invented lines. Two lists, because they mean
 * different things: notes being acted on, and things Work Studio noticed and is waiting to
 * be told about. The second influences nothing until the User agrees, which is the whole
 * point of showing it separately.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { Button, Field } from "./primitives.tsx";
import type { Note } from "../useOwn.ts";

export function SteeringPanel({
  notes,
  proposed,
  onAdd,
  onAct,
  problem,
}: {
  notes: Note[];
  proposed: Note[];
  onAdd: (note: string) => void;
  onAct: (id: string, action: "accept" | "reword" | "stop" | "forget", text?: string) => void;
  problem?: string;
}) {
  const [typed, setTyped] = useState("");

  return (
    <div style={{ display: "grid", gap: 12 }}>
      <div>
        <div className="h2" style={{ marginBottom: 8 }}>
          {t("thread.learned")}
        </div>

        {/* Anything Work Studio worked out for itself, asked as a question. */}
        {proposed.map((note) => (
          <Proposal key={note.id} note={note} onAct={onAct} />
        ))}

        {notes.length === 0 && proposed.length === 0 ? (
          <p style={{ fontSize: 12, color: "var(--muted)", margin: "0 0 8px" }}>
            {problem ?? t("steer.nothing_yet")}
          </p>
        ) : null}

        {notes.map((note) => (
          <Kept key={note.id} note={note} onAct={onAct} />
        ))}
      </div>

      <div>
        <Field
          placeholder={t("steer.new_placeholder")}
          value={typed}
          onChange={(event) => setTyped(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && typed.trim()) {
              event.preventDefault();
              onAdd(typed.trim());
              setTyped("");
            }
          }}
          style={{ fontSize: 12 }}
        />
        <p className="hint" style={{ marginTop: 7 }}>
          {t("steer.everything_i_go_on")}
        </p>
      </div>
    </div>
  );
}

/** Something Work Studio noticed. It acts on nothing until the User says so. */
function Proposal({
  note,
  onAct,
}: {
  note: Note;
  onAct: (id: string, action: "accept" | "reword" | "stop" | "forget", text?: string) => void;
}) {
  const [rewording, setRewording] = useState(false);
  const [text, setText] = useState(note.note);

  return (
    <div
      style={{
        border: "1px solid var(--border-strong)",
        background: "#fdfcf9",
        borderRadius: "var(--radius-sm)",
        padding: "10px 12px",
        marginBottom: 8,
      }}
    >
      <div style={{ fontSize: 12.5, marginBottom: 4 }}>{note.asks ?? note.note}</div>
      <div style={{ fontSize: 11, color: "var(--faint)", marginBottom: 8 }}>
        {note.provenance}
      </div>
      {rewording ? (
        <>
          <Field
            value={text}
            onChange={(event) => setText(event.target.value)}
            style={{ fontSize: 12, marginBottom: 7 }}
            label={t("steer.new_placeholder")}
          />
          <div style={{ display: "flex", gap: 6 }}>
            <Button small onClick={() => onAct(note.id, "reword", text)}>
              {t("steer.keep_this")}
            </Button>
            <Button small onClick={() => setRewording(false)}>
              {t("common.never_mind")}
            </Button>
          </div>
        </>
      ) : (
        <div style={{ display: "flex", gap: 6 }}>
          <Button small onClick={() => onAct(note.id, "accept")}>
            {t("steer.yes_do_that")}
          </Button>
          <Button small onClick={() => setRewording(true)}>
            {t("steer.reword")}
          </Button>
          <Button small onClick={() => onAct(note.id, "forget")}>
            {t("steer.no_thanks")}
          </Button>
        </div>
      )}
    </div>
  );
}

/** A note being acted on. */
function Kept({
  note,
  onAct,
}: {
  note: Note;
  onAct: (id: string, action: "accept" | "reword" | "stop" | "forget", text?: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(note.note);

  return (
    <div
      style={{
        display: "flex",
        gap: 8,
        alignItems: "flex-start",
        padding: "8px 0",
        borderTop: "1px solid #f2f0eb",
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        {editing ? (
          <Field
            value={text}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                onAct(note.id, "reword", text);
                setEditing(false);
              }
            }}
            style={{ fontSize: 12 }}
            label={t("steer.new_placeholder")}
          />
        ) : (
          <>
            <div style={{ fontSize: 12.5 }}>{note.note}</div>
            <div style={{ fontSize: 11, color: "var(--faint)", marginTop: 2 }}>
              {note.provenance}
            </div>
          </>
        )}
      </div>
      {editing ? (
        <Button
          small
          onClick={() => {
            onAct(note.id, "reword", text);
            setEditing(false);
          }}
        >
          {t("steer.keep_this")}
        </Button>
      ) : (
        <>
          <Button small onClick={() => setEditing(true)}>
            {t("details.edit")}
          </Button>
          <Button small onClick={() => onAct(note.id, "forget")}>
            {t("steer.forget")}
          </Button>
        </>
      )}
    </div>
  );
}
