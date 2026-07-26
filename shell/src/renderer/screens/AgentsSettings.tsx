/**
 * Settings › Agents.
 *
 * The one place in the product where the machinery is visible on purpose. A User who
 * opens Settings has gone looking for detail, so this section names agents, the model
 * behind each one, the tools it may use, its instructions, and what it remembers
 * (Requirement 23).
 *
 * Three things here are deliberate:
 *
 * **Tools are grouped by what they do to the world**, using the side-effect gate's own
 * classification — reads, edits your files, acts outside this computer. That is the
 * distinction that matters when deciding whether to revoke one, and it is the same
 * classification the gate enforces at run time, not a separate list that can drift.
 *
 * **Quality is measured from the tray.** Approving, editing before approving, and
 * rejecting are already graded human judgements the product collects. No synthetic
 * score, no labelling, and nothing the User has to trust on faith.
 *
 * **What cannot be answered is named.** Borrowed from the adk-rust console's
 * `TelemetryGap`: a figure we do not have is reported as unavailable with a reason and
 * an owner, never as zero.
 */

import { useState } from "react";

import {
  AGENTS,
  approvalRate,
  totalDecisions,
  type AgentProfile,
  type Effect,
  type MemoryFact,
} from "../agentFixtures.ts";
import { Button, Icon, Segmented } from "../components/primitives.tsx";

const EFFECT_LABEL: Record<Effect, string> = {
  reads: "Reads only",
  edits: "Changes your files",
  acts: "Acts outside this computer",
};

const EFFECT_TONE: Record<Effect, { bg: string; fg: string }> = {
  reads: { bg: "#eceae5", fg: "#5f5a53" },
  edits: { bg: "var(--info-bg)", fg: "var(--info-fg)" },
  acts: { bg: "var(--warn-bg)", fg: "var(--warn-fg)" },
};

import { AgentReach } from "./CapabilitiesPane.tsx";

export function AgentsSettings() {
  const [openId, setOpenId] = useState<string | undefined>();
  const open = AGENTS.find((agent) => agent.id === openId);

  if (open) {
    return <AgentDetail agent={open} onBack={() => setOpenId(undefined)} />;
  }

  return (
    <div>
      <p className="hint" style={{ margin: "-4px 0 16px" }}>
        Each piece of work is done by a specialist. You can see how each is doing, what
        it is allowed to use, and what it has learned.
      </p>

      <div className="stack">
        {AGENTS.map((agent) => (
          <button
            key={agent.id}
            type="button"
            onClick={() => setOpenId(agent.id)}
            style={{
              background: "var(--card)",
              border: "1px solid var(--border)",
              borderRadius: "var(--radius)",
              padding: "14px 16px",
              textAlign: "left",
              cursor: "pointer",
              font: "inherit",
              display: "flex",
              alignItems: "center",
              gap: 18,
            }}
          >
            <div style={{ minWidth: 0, flex: 1 }}>
              <div className="title">{agent.name}</div>
              <div className="sub">{agent.does}</div>
            </div>
            <Figure label="Accepted as-is" value={`${approvalRate(agent.outcomes)}%`} />
            <Sparkline values={agent.trend} />
            <Figure label="Typical wait" value={agent.latencyP50} />
            <Figure label="A day" value={agent.costPerDay} />
            <Icon name="chevronRight" size={15} stroke="var(--faint)" />
          </button>
        ))}
      </div>

      {/* What each specialist has actually been given, read from the store rather than
          listed here — so this cannot drift from what Settings changed. */}
      <div style={{ marginTop: 22 }}>
        <AgentReach />
      </div>
    </div>
  );
}

function AgentDetail({ agent, onBack }: { agent: AgentProfile; onBack: () => void }) {
  const [tier, setTier] = useState<string>(agent.tier);

  return (
    <div>
      <button
        type="button"
        onClick={onBack}
        style={{
          border: 0,
          background: "none",
          cursor: "pointer",
          color: "var(--muted)",
          fontSize: 12,
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: 0,
          marginBottom: 12,
        }}
      >
        <Icon name="chevronLeft" size={13} />
        All agents
      </button>

      <h2 style={{ fontSize: 16, fontWeight: 650, margin: "0 0 3px" }}>{agent.name}</h2>
      <p className="hint" style={{ margin: "0 0 18px" }}>
        {agent.does}
      </p>

      <Section title="How hard it thinks">
        <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
          <Segmented options={["Fast", "Balanced", "Best"]} active={tier} onSelect={setTier} />
          <span style={{ fontSize: 12, color: "var(--muted)" }}>
            Currently {agent.model}
          </span>
        </div>
        <p className="hint" style={{ marginTop: 8 }}>
          If that model is unavailable, the work is done by the next one configured and
          you get the same result.
        </p>
      </Section>

      <Section title="Its instructions">
        <div
          style={{
            border: "1px solid var(--border-strong)",
            borderRadius: 8,
            padding: "11px 13px",
            fontSize: 12.5,
            lineHeight: 1.6,
            color: "#3d3933",
            background: "#fdfdfc",
            whiteSpace: "pre-wrap",
          }}
        >
          {agent.prompt}
        </div>
        <div style={{ display: "flex", gap: 8, marginTop: 9, alignItems: "center" }}>
          <Button small>Edit</Button>
          <Button small>Reset to ours</Button>
          <span className="hint">
            Anything you tell it in How I should work is applied on top of this.
          </span>
        </div>
      </Section>

      <Section title="What it may use">
        {(["reads", "edits", "acts"] as Effect[]).map((effect) => {
          const tools = agent.tools.filter((tool) => tool.effect === effect);
          if (tools.length === 0) return null;
          return (
            <div key={effect} style={{ marginBottom: 12 }}>
              <span
                style={{
                  fontSize: 10.5,
                  fontWeight: 700,
                  letterSpacing: ".05em",
                  textTransform: "uppercase",
                  background: EFFECT_TONE[effect].bg,
                  color: EFFECT_TONE[effect].fg,
                  padding: "2px 7px",
                  borderRadius: 5,
                }}
              >
                {EFFECT_LABEL[effect]}
              </span>
              <div style={{ marginTop: 7 }}>
                {tools.map((tool) => (
                  <div
                    key={tool.name}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      padding: "7px 0",
                      borderBottom: "1px solid #f4f2ee",
                      fontSize: 12.5,
                    }}
                  >
                    <span style={{ minWidth: 150, color: "#3d3933" }}>{tool.what}</span>
                    <code
                      style={{
                        fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                        fontSize: 11,
                        color: "var(--faint)",
                      }}
                    >
                      {tool.name}
                    </code>
                    <span className="ml">
                      <Button small>{tool.granted ? "Allowed" : "Not allowed"}</Button>
                    </span>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
        <p className="hint">
          Anything that acts outside this computer still waits for your approval the
          first time, whatever is allowed here.
        </p>
      </Section>

      <Section title="What it remembers">
        <div
          style={{
            display: "flex",
            gap: 22,
            marginBottom: 12,
            fontSize: 12.5,
            flexWrap: "wrap",
          }}
        >
          <Stat label="Kept in mind while working" value={`${agent.shortTermTurns} turns`} />
          <Stat label="Sums up and starts fresh after" value={`${agent.compactAfter} turns`} />
          <Stat label="Things it has learned" value={`${agent.facts.length}`} />
        </div>
        {agent.facts.map((fact) => (
          <MemoryRow key={fact.id} fact={fact} />
        ))}
        <p className="hint" style={{ marginTop: 8 }}>
          It sums up long conversations by itself so that nothing important falls out of
          the end.
        </p>
      </Section>

      <Section title="How it's doing">
        <div style={{ display: "flex", gap: 26, flexWrap: "wrap", marginBottom: 14 }}>
          <Stat label="Accepted as-is" value={`${approvalRate(agent.outcomes)}%`} />
          <Stat label="You corrected it" value={`${agent.outcomes.approvedWithEdits}`} />
          <Stat label="You turned it down" value={`${agent.outcomes.rejected}`} />
          <Stat label="You undid it" value={`${agent.outcomes.undone}`} />
          <Stat label="Decisions counted" value={`${totalDecisions(agent.outcomes)}`} />
        </div>
        <div style={{ display: "flex", alignItems: "flex-end", gap: 14 }}>
          <Sparkline values={agent.trend} big />
          <span className="hint">Accepted as-is, over the last fortnight.</span>
        </div>
        <p className="hint" style={{ marginTop: 10 }}>
          This is measured from what you actually did — approving, correcting, turning
          down, undoing. Nothing here is a score it gave itself.
        </p>
      </Section>

      <Section title="Speed and cost">
        <div style={{ display: "flex", gap: 26, flexWrap: "wrap" }}>
          <Stat label="Half finish within" value={agent.latencyP50} />
          <Stat label="Nearly all within" value={agent.latencyP95} />
          <Stat label="A day" value={agent.costPerDay} />
          <Stat label="Fell back to another model" value={agent.failoverRate} />
        </div>
      </Section>

      <Section title="What we can't tell you yet" last>
        {agent.gaps.map((gap) => (
          <div
            key={gap.what}
            style={{
              display: "flex",
              gap: 10,
              alignItems: "flex-start",
              padding: "8px 0",
              borderBottom: "1px solid #f4f2ee",
              fontSize: 12.5,
            }}
          >
            <Icon name="info" size={13} stroke="var(--info-fg)" />
            <div style={{ flex: 1 }}>
              <div style={{ color: "#3d3933" }}>{gap.what}</div>
              <div style={{ fontSize: 11.5, color: "var(--faint)", marginTop: 2 }}>{gap.why}</div>
            </div>
            <span style={{ fontSize: 11.5, color: "var(--faint)" }}>{gap.owner}</span>
          </div>
        ))}
        <p className="hint" style={{ marginTop: 8 }}>
          A figure we do not have is shown as missing rather than as zero.
        </p>
      </Section>
    </div>
  );
}

function Section({
  title,
  children,
  last = false,
}: {
  title: string;
  children: React.ReactNode;
  last?: boolean;
}) {
  return (
    <section
      style={{
        padding: "16px 0",
        borderBottom: last ? 0 : "1px solid #f2f0eb",
      }}
    >
      <div
        style={{
          fontSize: 10.5,
          fontWeight: 700,
          letterSpacing: ".07em",
          textTransform: "uppercase",
          color: "var(--faint)",
          marginBottom: 10,
        }}
      >
        {title}
      </div>
      {children}
    </section>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div style={{ fontSize: 11.5, color: "var(--muted)" }}>{label}</div>
      <div style={{ fontSize: 17, fontWeight: 640, letterSpacing: "-.01em", marginTop: 2 }}>
        {value}
      </div>
    </div>
  );
}

function Figure({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ textAlign: "right", minWidth: 74 }}>
      <div style={{ fontSize: 10.5, color: "var(--faint)" }}>{label}</div>
      <div style={{ fontSize: 14, fontWeight: 640 }}>{value}</div>
    </div>
  );
}

function Sparkline({ values, big = false }: { values: number[]; big?: boolean }) {
  const width = big ? 260 : 84;
  const height = big ? 46 : 22;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const points = values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * width;
      const y = height - ((value - min) / span) * (height - 2) - 1;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  return (
    <svg
      width={width}
      height={height}
      role="img"
      aria-label={`Accepted as-is, ${values[0]}% to ${values[values.length - 1]}% over a fortnight`}
      style={{ flex: "0 0 auto" }}
    >
      <polyline
        points={points}
        fill="none"
        stroke="var(--live-fg)"
        strokeWidth={big ? 2 : 1.5}
        strokeLinejoin="round"
      />
    </svg>
  );
}

function MemoryRow({ fact }: { fact: MemoryFact }) {
  return (
    <div
      style={{
        display: "flex",
        gap: 11,
        alignItems: "flex-start",
        padding: "8px 0",
        borderBottom: "1px solid #f4f2ee",
        fontSize: 12.5,
      }}
    >
      <span
        style={{
          fontSize: 10.5,
          fontWeight: 700,
          letterSpacing: ".05em",
          textTransform: "uppercase",
          color: "var(--ink-soft)",
          background: "#f1efea",
          padding: "2px 7px",
          borderRadius: 5,
          whiteSpace: "nowrap",
        }}
      >
        {fact.kind}
      </span>
      <div style={{ flex: 1 }}>
        <div style={{ color: "#3d3933" }}>{fact.text}</div>
        <div style={{ fontSize: 11.5, color: "var(--faint)", marginTop: 2 }}>
          {fact.learnedFrom}
        </div>
      </div>
      <Button small>Forget</Button>
    </div>
  );
}
