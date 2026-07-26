/**
 * Dashboard.
 *
 * The metric strip, then both trays side by side, then the work that runs on its
 * own. Cost sits on the metric strip rather than in Settings: autonomous work with
 * no visible cost is the fastest route to distrust.
 */

import { t } from "../../shared/strings.ts";
import { DELIVERIES, THREADS, TRAY } from "../fixtures.ts";
import { useOverview } from "../useOwn.ts";
import { Button, Card, Pill } from "../components/primitives.tsx";
import { TrayRow } from "../components/state.tsx";
import type { Route } from "../routes.ts";

export function Dashboard({
  onNavigate,
}: {
  onNavigate: (route: Route, threadId?: string) => void;
}) {
  const overview = useOverview();
  const waiting = TRAY.slice(0, 3);
  const done = DELIVERIES.slice(0, 3);
  const scheduled = THREADS.filter((thread) =>
    ["scheduled", "needsYou", "working"].includes(thread.badge),
  ).slice(0, 3);

  return (
    <main className="main">
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: 12,
          marginBottom: 22,
        }}
      >
        {/* Counted, not invented. A figure Work Studio cannot answer shows a dash and is
            marked unknown, because zero is a claim that nothing happened. */}
        <Metric
          label={t("dash.metric.working")}
          value={overview.working.value}
          known={overview.working.known}
        />
        <Metric
          label={t("dash.metric.waiting")}
          value={overview.waiting.value}
          known={overview.waiting.known}
        />
        <Metric
          label={t("dash.metric.done")}
          value={overview.done.value}
          known={overview.done.known}
        />
        <Metric
          label={t("dash.metric.cost")}
          value={overview.cost.value}
          known={overview.cost.known}
          note={overview.note}
        />
      </div>

      <div className="cols">
        <section aria-label={t("dash.waiting_heading")}>
          <div className="h2">{t("dash.waiting_heading")}</div>
          <div className="stack">
            {waiting.map((item) => (
              <TrayRow
                key={item.id}
                cls={item.cls}
                headline={item.headline}
                detail={item.detail}
                actions={
                  <Button
                    onClick={() =>
                      onNavigate(item.cls === "kickoff" ? "kickoffOutput" : "tray")
                    }
                  >
                    {item.choices[0]}
                  </Button>
                }
              />
            ))}
          </div>
        </section>

        <section aria-label={t("dash.done_heading")}>
          <div className="h2">{t("dash.done_heading")}</div>
          <div className="stack">
            {done.map((delivery) => (
              <Card key={delivery.id}>
                <div className="row">
                  <div>
                    <div className="title">{delivery.action}</div>
                    <div className="sub">
                      {delivery.when} · {delivery.thread}
                      {delivery.extra ? ` · ${delivery.extra}` : ""}
                    </div>
                  </div>
                  {delivery.reversal.kind === "available" ? (
                    <Button small style={{ marginLeft: "auto" }}>
                      {delivery.reversal.label}
                    </Button>
                  ) : null}
                </div>
              </Card>
            ))}
            <button
              type="button"
              onClick={() => onNavigate("outTray")}
              style={{
                border: 0,
                background: "none",
                color: "#5f5a53",
                fontSize: 12,
                textDecoration: "underline",
                textDecorationColor: "#d5d0c7",
                textUnderlineOffset: 2,
                cursor: "pointer",
                alignSelf: "flex-start",
                padding: "2px 0",
              }}
            >
              {t("out.heading")}
            </button>
          </div>
        </section>
      </div>

      <div className="h2" style={{ marginTop: 20 }}>
        {t("dash.running_heading")}
      </div>
      <div className="grid3">
        {scheduled.map((thread) => (
          <button
            key={thread.id}
            type="button"
            onClick={() => onNavigate("thread", thread.id)}
            style={{
              background: "var(--card)",
              border: "1px solid var(--border)",
              borderRadius: "var(--radius)",
              padding: "13px 15px",
              textAlign: "left",
              cursor: "pointer",
              font: "inherit",
            }}
          >
            <div className="title">{thread.purpose}</div>
            <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 6 }}>
              {thread.badge === "needsYou" ? (
                <Pill tone="warn" icon="warning">
                  Paused · Gmail
                </Pill>
              ) : thread.badge === "working" ? (
                <Pill tone="live">Working</Pill>
              ) : (
                <Pill>Waiting for its time</Pill>
              )}
              <span style={{ fontSize: 12, color: "var(--faint)" }}>
                {thread.nextHuman ?? thread.statusDetail}
              </span>
            </div>
          </button>
        ))}
      </div>
    </main>
  );
}

function Metric({
  label,
  value,
  known = true,
  note,
}: {
  label: string;
  value: string;
  /** False when this was not measured. Shown differently, so it cannot read as a count. */
  known?: boolean;
  note?: string;
}) {
  return (
    <div
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        borderRadius: "var(--radius)",
        padding: "13px 15px",
      }}
      title={known ? undefined : note}
    >
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.3, minHeight: 31 }}>
        {label}
      </div>
      <div
        style={{
          fontSize: 25,
          fontWeight: 640,
          letterSpacing: "-.02em",
          marginTop: 2,
          // A figure nobody measured is drawn quietly, so it does not sit on the screen with
          // the same authority as one that was counted.
          color: known ? undefined : "var(--faint)",
        }}
      >
        {value}
      </div>
      {known ? null : (
        <div style={{ fontSize: 10.5, color: "var(--faint)", marginTop: 2 }}>
          {t("dash.not_measured")}
        </div>
      )}
    </div>
  );
}
