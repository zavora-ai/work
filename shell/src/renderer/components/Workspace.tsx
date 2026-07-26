/**
 * The three-zone workspace.
 *
 * Navigation left, the document in the centre, tools right — the arrangement every
 * office application has settled on. The centre is the file itself on a workspace
 * backdrop, so it reads as a page in a frame rather than as another panel of
 * application chrome.
 *
 * The right panel's home state is the conversation. Details and Format swap in over
 * it and return. If chat could be displaced permanently it would be hidden exactly
 * when the User needed it.
 *
 * Both rails collapse. Collapsed, this is a document editor and nothing else — which
 * is the point: the User should not need a second office suite open.
 */

import { useState, type ReactNode } from "react";

import { t } from "../../shared/strings.ts";
import { Button, Field, Icon } from "./primitives.tsx";

export type Pane = "chat" | "details" | "format";

export function Workspace({
  fileName,
  toolbar,
  status,
  canvas,
  fill,
  footer,
  conversation,
  details,
  pane,
  onPane,
  rightCollapsed,
  onToggleRight,
}: {
  fileName: string;
  toolbar: ReactNode;
  status?: ReactNode;
  canvas: ReactNode;
  /**
   * Let the canvas own the whole area: no padding, no centring, full height.
   *
   * A page or a slide is a sheet of paper on a desk, so it sits in the middle with room
   * around it. A spreadsheet is not — it is the surface itself, and margins around it
   * only waste the space the User needs for rows and columns.
   */
  fill?: boolean;
  footer?: ReactNode;
  conversation: ReactNode;
  details: ReactNode;
  pane: Pane;
  onPane: (pane: Pane) => void;
  rightCollapsed: boolean;
  onToggleRight: () => void;
}) {
  return (
    <>
      <div
        style={{
          flex: 1,
          minWidth: 0,
          background: "var(--workspace)",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 5,
            padding: "8px 12px",
            background: "var(--card)",
            borderBottom: "1px solid var(--border)",
          }}
        >
          <span style={{ fontSize: 12.5, fontWeight: 650, marginRight: 6 }}>{fileName}</span>
          {toolbar}
          <div className="ml" style={{ display: "flex", gap: 6, alignItems: "center" }}>
            {status}
            <Button small onClick={() => onPane("details")}>
              {t("doc.details")}
            </Button>
          </div>
        </div>

        <div
          style={
            fill
              ? {
                  flex: 1,
                  minHeight: 0,
                  display: "flex",
                  flexDirection: "column",
                  overflow: "hidden",
                }
              : {
                  flex: 1,
                  overflow: "auto",
                  padding: 18,
                  display: "flex",
                  justifyContent: "center",
                }
          }
        >
          {canvas}
        </div>
        {footer ? <div style={{ padding: "0 18px 12px" }}>{footer}</div> : null}
      </div>

      {rightCollapsed ? (
        <div
          style={{
            width: 44,
            background: "var(--card)",
            borderLeft: "1px solid var(--border)",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            paddingTop: 12,
            gap: 14,
            flex: "0 0 auto",
          }}
        >
          <button
            type="button"
            onClick={onToggleRight}
            title={t("nav.expand")}
            aria-label={t("nav.expand")}
            style={{ border: 0, background: "none", cursor: "pointer", color: "#8a8580" }}
          >
            <Icon name="chevronLeft" size={16} />
          </button>
          <Icon name="chat" size={15} stroke="var(--faint)" />
          <Icon name="info" size={15} stroke="var(--faint)" />
        </div>
      ) : (
        <aside
          style={{
            width: 268,
            background: "var(--card)",
            borderLeft: "1px solid var(--border)",
            display: "flex",
            flexDirection: "column",
            flex: "0 0 auto",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 7,
              padding: "11px 12px",
              borderBottom: "1px solid var(--border)",
              fontSize: 11,
              fontWeight: 700,
              letterSpacing: ".07em",
              textTransform: "uppercase",
              color: "#6d6862",
            }}
          >
            {pane === "chat" ? t("doc.chat") : pane === "details" ? t("doc.details") : t("doc.format")}
            <button
              type="button"
              onClick={onToggleRight}
              title={t("nav.collapse")}
              aria-label={t("nav.collapse")}
              className="ml"
              style={{ border: 0, background: "none", cursor: "pointer", color: "#a29d94" }}
            >
              <Icon name="chevronRight" size={14} />
            </button>
          </div>

          <div style={{ padding: 8 }}>
            <div
              role="tablist"
              style={{ display: "flex", gap: 2, background: "#f1efea", borderRadius: 7, padding: 2 }}
            >
              {(["chat", "details", "format"] as Pane[]).map((option) => {
                const label =
                  option === "chat" ? t("doc.chat") : option === "details" ? t("doc.details") : t("doc.format");
                const on = option === pane;
                return (
                  <button
                    key={option}
                    type="button"
                    role="tab"
                    aria-selected={on}
                    onClick={() => onPane(option)}
                    style={{
                      flex: 1,
                      border: 0,
                      fontSize: 11.5,
                      fontWeight: 600,
                      color: on ? "#26241f" : "#6d6862",
                      padding: "4px 0",
                      borderRadius: 5,
                      background: on ? "#fff" : "transparent",
                      boxShadow: on ? "0 1px 2px rgba(0,0,0,.06)" : "none",
                      cursor: "pointer",
                    }}
                  >
                    {label}
                  </button>
                );
              })}
            </div>
          </div>

          <div style={{ flex: 1, overflowY: "auto", padding: pane === "details" ? 0 : 8 }}>
            {pane === "details" ? details : pane === "chat" ? conversation : <FormatPane />}
          </div>
        </aside>
      )}
    </>
  );
}

/* -------------------------------------------------------------- conversation */

export function Bubble({ from, children }: { from: "you" | "studio"; children: ReactNode }) {
  const you = from === "you";
  return (
    <div
      style={{
        maxWidth: "100%",
        alignSelf: you ? "flex-end" : "flex-start",
        background: you ? "#26241f" : "#fff",
        color: you ? "#fff" : "inherit",
        border: you ? 0 : "1px solid var(--border)",
        borderRadius: 11,
        borderBottomRightRadius: you ? 4 : 11,
        borderBottomLeftRadius: you ? 11 : 4,
        padding: "9px 12px",
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      {children}
    </div>
  );
}

/** The turn-level summary of what changed, with undo. */
export function ChangeCard({ summary }: { summary: string }) {
  return (
    <div
      style={{
        border: "1px solid var(--border)",
        borderRadius: 9,
        padding: "9px 10px",
        background: "#fbfaf8",
      }}
    >
      <div style={{ fontSize: 11.5, fontWeight: 650, marginBottom: 5 }}>{summary}</div>
      <div style={{ display: "flex", gap: 6 }}>
        <Button small>{t("out.undo")}</Button>
        <Button small>{t("details.review_all")}</Button>
      </div>
    </div>
  );
}

export function Conversation({ children }: { children: ReactNode }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 9, height: "100%" }}>
      {children}
      <div style={{ marginTop: "auto" }}>
        <Field placeholder={t("doc.ask_change")} style={{ fontSize: 12 }} />
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ details */

export function DetailsSection({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ padding: "10px 12px", borderBottom: "1px solid #f0eeE9" }}>
      <div
        style={{
          fontSize: 10.5,
          fontWeight: 700,
          letterSpacing: ".06em",
          textTransform: "uppercase",
          color: "var(--faint)",
          marginBottom: 7,
        }}
      >
        {label}
      </div>
      {children}
    </div>
  );
}

export function DetailsLine({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        fontSize: 12,
        color: "#3d3933",
        display: "flex",
        gap: 7,
        alignItems: "flex-start",
        marginBottom: 6,
        lineHeight: 1.4,
      }}
    >
      {children}
    </div>
  );
}

function FormatPane() {
  return (
    <div className="hint" style={{ padding: 4 }}>
      Formatting controls appear here for whatever you have selected.
    </div>
  );
}
