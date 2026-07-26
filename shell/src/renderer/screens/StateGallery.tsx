/**
 * Every state on one surface.
 *
 * Not a User-facing screen. It exists so that progress, empty and failure states can
 * be reviewed and screenshotted, because these are exactly the states that never
 * appear in a demo and are therefore found in use rather than in review.
 */

import { t } from "../../shared/strings.ts";
import { Button } from "../components/primitives.tsx";
import { Empty, Failure, Progress, Working } from "../components/states.tsx";

export function StateGallery() {
  return (
    <main className="main">
      <h1 className="h1">States</h1>
      <p className="hint" style={{ margin: "-6px 0 20px" }}>
        For review. Progress, emptiness and the four kinds of failure.
      </p>

      <Group label="While it works">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 18 }}>
          <Progress
            steps={[
              "Reading your sources — found 34 items",
              "Picking what matters",
              "Writing it up",
              "Checking it reads well",
            ]}
            current={1}
          />
          <div className="stack">
            <Working what="Building 8 slides…" />
            <Working what="Recalculating 240 cells…" />
            <p className="hint">
              A wait longer than a moment always says what it is doing, and what it
              found so far.
            </p>
          </div>
        </div>
      </Group>

      <Group label="When there is nothing there">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 18 }}>
          <Empty
            icon="check"
            title={t("dash.nothing_waiting")}
            what="Anything that needs a decision from you will appear here. Nothing expires while it waits."
          />
          <Empty
            icon="folder"
            title="No files yet"
            what="Files you and I make together live in Documents › Work Studio, as ordinary files you can open anywhere."
            action={<Button small>{t("new.title")}</Button>}
          />
          <Empty
            icon="bolt"
            title="Nothing runs on its own yet"
            what="Hand something over and I'll do it on a schedule — a morning digest, inbox triage, a health check."
            action={<Button small>{t("new.recurring")}</Button>}
          />
          <Empty
            icon="sheet"
            title="This sheet is empty"
            what="Ask for what you need and I'll build it, or type straight into a cell."
          />
        </div>
      </Group>

      <Group label="When something goes wrong">
        <div className="stack">
          <p className="hint">
            A failure that fixed itself shows nothing at all — the class below renders
            nothing, deliberately.
          </p>
          <Failure kind="recovered" headline="(nothing is shown)" />

          <Failure
            kind="userActionable"
            headline="Gmail needs reconnecting"
            detail="Your sign-in expired on Thursday. Inbox triage and Morning digest are paused until you reconnect."
            action={t("fail.reconnect")}
          />
          <Failure
            kind="stoppedTrying"
            headline="Your daily newsletter didn't send"
            detail="The same thing went wrong three mornings running, so I've paused it."
            action="Look at it"
          />
          <Failure
            kind="internal"
            headline={t("fail.internal")}
            detail="Nothing was lost. If it keeps happening, Technical details in Settings has what support needs."
          />
        </div>
      </Group>
    </main>
  );
}

function Group({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <section style={{ marginBottom: 26 }}>
      <div className="h2">{label}</div>
      {children}
    </section>
  );
}
