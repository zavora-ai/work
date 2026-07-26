/**
 * Dashboard.
 *
 * The metric strip, then both trays side by side, then the work that runs on its
 * own. Cost sits on the metric strip rather than in Settings: autonomous work with
 * no visible cost is the fastest route to distrust.
 */

import { t } from "../../shared/strings.ts";
import { DELIVERIES, METRICS, THREADS, TRAY } from "../fixtures.ts";
import { Button, Card, Pill } from "../components/primitives.tsx";
import { TrayRow } from "../components/state.tsx";
import type { Route } from "../routes.ts";

export function Dashboard({
  onNavigate,
}: {
  onNavigate: (route: Route, threadId?: string) => void;
}) {
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
        <Metric label={t("dash.metric.working")} value={METRICS.working} />
        <Metric label={t("dash.metric.waiting")} value={METRICS.waiting} />
        <Metric label={t("dash.metric.done")} value={METRICS.done} />
        <Metric label={t("dash.metric.cost")} value={METRICS.cost} />
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

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div
      style={{
        background: "var(--card)",
        border: "1px solid var(--border)",
        borderRadius: "var(--radius)",
        padding: "13px 15px",
      }}
    >
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.3, minHeight: 31 }}>
        {label}
      </div>
      <div style={{ fontSize: 25, fontWeight: 640, letterSpacing: "-.02em", marginTop: 2 }}>
        {value}
      </div>
    </div>
  );
}
