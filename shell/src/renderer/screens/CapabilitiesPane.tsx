/**
 * What each specialist can reach.
 *
 * The Core has been able to answer this for a while and nothing asked it, so a User could not
 * see or change any of it. This is that screen.
 *
 * Two words are avoided on purpose. It never says "server", because the vocabulary rule
 * forbids that everywhere including here, and it never shows a path or the value of anything a
 * connection needs — the Core does not send either, since a value may be a credential.
 */

import { useState } from "react";

import { t } from "../../shared/strings.ts";
import { Button, Field, SectionLabel } from "../components/primitives.tsx";
import { useCapabilities, type Capability } from "../useOwn.ts";

/** The specialists a connection can be given to, in the User's words. */
const SPECIALISTS: { id: string; label: string }[] = [
  { id: "spreadsheet", label: t("repo.kind.spreadsheets") },
  { id: "document", label: t("repo.kind.documents") },
  { id: "presentation", label: t("repo.kind.decks") },
];

function tone(readiness: string): { bg: string; fg: string } {
  if (readiness === "ready") return { bg: "#e8f3ec", fg: "#1f6b43" };
  if (readiness === "missing") return { bg: "#fdf1e7", fg: "#8a4b16" };
  return { bg: "#f1f0ec", fg: "#6b675f" };
}

export function CapabilitiesPane() {
  const { items, problem, act, add } = useCapabilities();
  const [adding, setAdding] = useState(false);
  const [label, setLabel] = useState("");
  const [command, setCommand] = useState("");

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div>
        <div className="h2" style={{ marginBottom: 4 }}>
          {t("caps.title")}
        </div>
        <p className="hint" style={{ margin: 0 }}>
          {t("caps.intro")}
        </p>
      </div>

      {problem ? (
        <p style={{ fontSize: 12, color: "var(--muted)", margin: 0 }}>{problem}</p>
      ) : null}

      {items.length === 0 && !problem ? (
        <p style={{ fontSize: 12, color: "var(--muted)", margin: 0 }}>{t("caps.none")}</p>
      ) : null}

      <div style={{ display: "grid", gap: 10 }}>
        {items.map((item) => (
          <Row key={item.id} item={item} onAct={act} />
        ))}
      </div>

      {adding ? (
        <div
          style={{
            border: "1px solid var(--border-strong)",
            borderRadius: "var(--radius-sm)",
            padding: 12,
            display: "grid",
            gap: 8,
          }}
        >
          <Field
            placeholder={t("caps.name_placeholder")}
            value={label}
            label={t("caps.name_placeholder")}
            onChange={(event) => setLabel(event.target.value)}
            style={{ fontSize: 12 }}
          />
          <Field
            placeholder={t("caps.command_placeholder")}
            value={command}
            label={t("caps.command_placeholder")}
            onChange={(event) => setCommand(event.target.value)}
            style={{ fontSize: 12 }}
          />
          <div style={{ display: "flex", gap: 6 }}>
            <Button
              small
              onClick={() => {
                if (!label.trim() || !command.trim()) return;
                void add(label.trim(), command.trim(), []);
                setLabel("");
                setCommand("");
                setAdding(false);
              }}
            >
              {t("caps.add")}
            </Button>
            <Button small onClick={() => setAdding(false)}>
              {t("common.never_mind")}
            </Button>
          </div>
          <p className="hint" style={{ margin: 0 }}>
            {t("caps.then_allocate")}
          </p>
        </div>
      ) : (
        <div>
          <Button small onClick={() => setAdding(true)}>
            {t("caps.add_one")}
          </Button>
        </div>
      )}
    </div>
  );
}

function Row({
  item,
  onAct,
}: {
  item: Capability;
  onAct: (id: string, action: "on" | "off" | "remove" | "allocate", agents?: string[]) => void;
}) {
  const colour = tone(item.readiness);
  const off = item.readiness === "off";

  /** Give it to a specialist, or take it away. Sends the whole set, as the Core expects. */
  const toggleSpecialist = (which: string) => {
    const next = item.agents.includes(which)
      ? item.agents.filter((a) => a !== which)
      : [...item.agents, which];
    onAct(item.id, "allocate", next);
  };

  return (
    <div
      style={{
        border: "1px solid var(--border)",
        borderRadius: "var(--radius-sm)",
        background: "var(--card)",
        padding: "11px 13px",
        display: "grid",
        gap: 9,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 9 }}>
        <span style={{ fontSize: 12.5, fontWeight: 600 }}>{item.label}</span>
        <span
          style={{
            fontSize: 10.5,
            fontWeight: 650,
            padding: "2px 7px",
            borderRadius: 20,
            background: colour.bg,
            color: colour.fg,
          }}
        >
          {item.status}
        </span>
        <div className="ml" style={{ display: "flex", gap: 6 }}>
          <Button small onClick={() => onAct(item.id, off ? "on" : "off")}>
            {off ? t("caps.turn_on") : t("caps.turn_off")}
          </Button>
          {/* What came with Work Studio may be turned off but not removed: the product
              depends on it, and offering removal would let the User break their own files
              in a way they could not undo from here. */}
          {item.builtIn ? null : (
            <Button small onClick={() => onAct(item.id, "remove")}>
              {t("caps.remove")}
            </Button>
          )}
        </div>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 7, flexWrap: "wrap" }}>
        <span style={{ fontSize: 11, color: "var(--faint)" }}>{t("caps.used_by")}</span>
        {SPECIALISTS.map((specialist) => {
          const on = item.agents.includes(specialist.id);
          return (
            <button
              key={specialist.id}
              type="button"
              aria-pressed={on}
              onClick={() => toggleSpecialist(specialist.id)}
              style={{
                fontSize: 11,
                padding: "3px 9px",
                borderRadius: 20,
                cursor: "pointer",
                border: on ? "1px solid var(--border-strong)" : "1px solid var(--border)",
                background: on ? "#eef0f4" : "transparent",
                color: on ? "#26241f" : "var(--muted)",
                fontWeight: on ? 650 : 500,
              }}
            >
              {specialist.label}
            </button>
          );
        })}
        {item.agents.length === 0 ? (
          <span style={{ fontSize: 11, color: "var(--muted)" }}>{t("caps.nobody")}</span>
        ) : null}
      </div>

      {item.needs.length > 0 ? (
        <div style={{ fontSize: 11, color: "var(--faint)" }}>
          {t("caps.needs")} {item.needs.join(", ")}
        </div>
      ) : null}
    </div>
  );
}

/** The Agents section's view of the same thing: what each specialist has been given. */
export function AgentReach() {
  const { items } = useCapabilities();

  return (
    <div style={{ display: "grid", gap: 12 }}>
      <SectionLabel>{t("caps.per_agent")}</SectionLabel>
      {SPECIALISTS.map((specialist) => {
        const given = items.filter(
          (item) => item.agents.includes(specialist.id) && item.readiness === "ready",
        );
        const idle = items.filter(
          (item) => item.agents.includes(specialist.id) && item.readiness !== "ready",
        );
        return (
          <div key={specialist.id} style={{ fontSize: 12 }}>
            <div style={{ fontWeight: 600, marginBottom: 3 }}>{specialist.label}</div>
            <div style={{ color: "var(--muted)" }}>
              {given.length === 0
                ? t("caps.nothing_given")
                : given.map((item) => item.label).join(", ")}
              {idle.length > 0 ? (
                <span style={{ color: "var(--faint)" }}>
                  {" "}
                  · {idle.map((item) => `${item.label} (${item.status})`).join(", ")}
                </span>
              ) : null}
            </div>
          </div>
        );
      })}
    </div>
  );
}
