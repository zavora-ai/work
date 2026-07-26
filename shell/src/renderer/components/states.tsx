/**
 * The states between nothing and finished.
 *
 * Three families of state that a screenshot of a working product never shows, and
 * which are therefore the easiest to leave until they are discovered in use:
 *
 * **Progress.** Requirement 19.3 forbids an unexplained wait longer than three
 * seconds. Progress is reported in the User's terms — "Reading your sources… found
 * 34 items" — never as a percentage of something they cannot see.
 *
 * **Empty.** An empty surface must say what it is for, not just be blank. Every one
 * of these is reachable on a real first day.
 *
 * **Failure.** Requirement 17 sorts failures into four classes, and the class decides
 * the treatment: a recovered failure is silent, a User-actionable one names the
 * account and offers one action, a repeated failure says it has stopped trying, and an
 * internal fault reassures without explaining itself.
 */

import type { ReactNode } from "react";

import { t } from "../../shared/strings.ts";
import { Button, Icon, type IconName } from "./primitives.tsx";

/* ----------------------------------------------------------------- progress */

/** A step in the User's language, with what it found. */
export function Progress({ steps, current }: { steps: string[]; current: number }) {
  return (
    <div
      style={{
        border: "1px solid var(--border)",
        borderRadius: 10,
        background: "#fbfaf8",
        padding: "11px 13px",
      }}
    >
      {steps.map((step, index) => {
        const done = index < current;
        const active = index === current;
        return (
          <div
            key={step}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 9,
              fontSize: 12.5,
              padding: "3px 0",
              color: done ? "var(--muted)" : active ? "#26241f" : "var(--faint)",
              fontWeight: active ? 600 : 400,
            }}
          >
            {done ? (
              <Icon name="check" size={13} stroke="var(--live-fg)" width={2.6} />
            ) : active ? (
              <Spinner />
            ) : (
              <span
                style={{
                  width: 13,
                  height: 13,
                  borderRadius: "50%",
                  border: "1.5px solid var(--border-strong)",
                  flex: "0 0 auto",
                }}
              />
            )}
            {step}
          </div>
        );
      })}
    </div>
  );
}

function Spinner() {
  return (
    <span
      aria-hidden="true"
      style={{
        width: 13,
        height: 13,
        flex: "0 0 auto",
        borderRadius: "50%",
        border: "1.5px solid var(--working-halo)",
        borderTopColor: "var(--working)",
        animation: "zws-spin .8s linear infinite",
      }}
    />
  );
}

/** A one-line working indicator for a toolbar. */
export function Working({ what }: { what: string }) {
  return (
    <span
      role="status"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 7,
        fontSize: 11.5,
        color: "var(--muted)",
      }}
    >
      <Spinner />
      {what}
    </span>
  );
}

/* -------------------------------------------------------------------- empty */

export function Empty({
  icon,
  title,
  what,
  action,
}: {
  icon: IconName;
  title: string;
  what: string;
  action?: ReactNode;
}) {
  return (
    <div
      style={{
        border: "1px dashed var(--border-strong)",
        borderRadius: "var(--radius)",
        padding: "34px 26px",
        textAlign: "center",
        background: "rgba(255,255,255,.5)",
      }}
    >
      <div style={{ display: "grid", placeItems: "center", marginBottom: 10 }}>
        <Icon name={icon} size={22} stroke="var(--faint)" width={1.6} />
      </div>
      <div style={{ fontSize: 13.5, fontWeight: 600, marginBottom: 4 }}>{title}</div>
      <p className="hint" style={{ maxWidth: 380, margin: "0 auto" }}>
        {what}
      </p>
      {action ? <div style={{ marginTop: 14 }}>{action}</div> : null}
    </div>
  );
}

/* ------------------------------------------------------------------ failure */

export type FailureClass = "recovered" | "userActionable" | "stoppedTrying" | "internal";

/**
 * A failure, in the treatment its class dictates.
 *
 * `recovered` renders nothing at all — that is the whole point of the class. It is
 * included so the exhaustive switch is visible rather than implied.
 */
export function Failure({
  kind,
  headline,
  detail,
  action,
}: {
  kind: FailureClass;
  headline: string;
  detail?: string;
  action?: string;
}) {
  if (kind === "recovered") return null;

  const tone =
    kind === "internal"
      ? { bg: "#f4f2ee", fg: "var(--ink-soft)", edge: "var(--border-strong)", icon: "info" as IconName }
      : { bg: "var(--warn-bg)", fg: "var(--warn-fg)", edge: "var(--warn-edge)", icon: "warning" as IconName };

  return (
    <div
      role="alert"
      style={{
        background: tone.bg,
        border: "1px solid var(--border)",
        borderLeft: `3px solid ${tone.edge}`,
        borderRadius: "var(--radius)",
        padding: "12px 14px",
        display: "flex",
        gap: 11,
        alignItems: "flex-start",
      }}
    >
      <Icon name={tone.icon} size={15} stroke={tone.fg} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 13, fontWeight: 600 }}>{headline}</div>
        {detail ? (
          <div style={{ fontSize: 12.5, color: "var(--ink-soft)", marginTop: 3, lineHeight: 1.5 }}>
            {detail}
            {kind === "stoppedTrying" ? ` ${t("fail.stopped_trying")}` : ""}
          </div>
        ) : null}
      </div>
      {action ? <Button small>{action}</Button> : null}
    </div>
  );
}
