/**
 * The three artefact clients.
 *
 * Each renders the real file: a document as HTML from the layout engine, a deck as
 * SVG per slide, a spreadsheet as a grid with a formula bar. What Work Studio changed
 * itself is attributed and individually reversible, so the change log is a product
 * feature rather than an audit artefact.
 *
 * The ladder is deliberate. Common operations are offered directly; anything rarer is
 * asked for in the conversation. Someone who wanted to build a deck by hand would
 * have opened PowerPoint.
 */

import React, { useState } from "react";

import { useSheet } from "../useSheet.ts";
import type { DeckState, DocumentState } from "../useArtefact.ts";
import { t } from "../../shared/strings.ts";
import { Button, Card, Field, Icon } from "../components/primitives.tsx";
import { SheetGrid } from "../components/SheetGrid.tsx";
import { Failure, Progress } from "../components/states.tsx";
import {
  Bubble,
  ChangeCard,
  Conversation,
  DetailsLine,
  DetailsSection,
  Workspace,
  type Pane,
} from "../components/Workspace.tsx";

interface WorkspaceProps {
  pane: Pane;
  onPane: (pane: Pane) => void;
  rightCollapsed: boolean;
  onToggleRight: () => void;
}

/* ---------------------------------------------------------------- document */

export function DocumentWorkspace(props: WorkspaceProps & { state: DocumentState }) {
  const doc = props.state;
  const [selected, setSelected] = useState<number | undefined>(undefined);

  // A click anywhere in the page tells us which block it was, because the Core marked
  // every one. That is what makes a change attributable to a paragraph rather than to
  // "the document".
  const onCanvasClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const target = event.target as HTMLElement | null;
    const block = target?.closest("[data-p]");
    const raw = block?.getAttribute("data-p");
    setSelected(raw === null || raw === undefined ? undefined : Number(raw));
  };

  return (
    <Workspace
      {...props}
      fileName={doc.model?.fileName ?? "Document"}
      toolbar={
        <>
          <Button small style={{ fontWeight: 700 }}>
            B
          </Button>
          <Button small style={{ fontStyle: "italic" }}>
            I
          </Button>
          <Button small>Heading</Button>
          <Button small>List</Button>
          <Button small>Table</Button>
          <Button small>Comment</Button>
        </>
      }
      status={
        <span style={{ fontSize: 11.5, color: "var(--muted)", fontWeight: 600 }}>
          {doc.problem
            ? doc.problem
            : selected === undefined
              ? `${doc.model?.blockCount ?? 0} ${t("doc.blocks")}`
              : t("doc.selected_block")}
        </span>
      }
      canvas={
        doc.problem ? (
          <div style={{ padding: 24, color: "var(--muted)", maxWidth: 460 }}>{doc.problem}</div>
        ) : (
          <div
            onClick={onCanvasClick}
            style={{
              background: "#fff",
              border: "1px solid #dedad2",
              boxShadow: "0 1px 4px rgba(0,0,0,.05)",
              width: 560,
              padding: "34px 44px",
              fontSize: 12.5,
              lineHeight: 1.7,
              color: "#2b2823",
              alignSelf: "flex-start",
            }}
          >
            {doc.model ? (
              <div
                // The Core produced this markup from the User's own file. It is the
                // editable view, and every block in it carries its identifier.
                dangerouslySetInnerHTML={{ __html: doc.model.html }}
              />
            ) : (
              <div style={{ color: "var(--muted)" }}>{t("common.loading")}</div>
            )}
          </div>
        )
      }
      conversation={
        <Conversation>
          <Bubble from="you">Tighten the termination clause</Bubble>
          <Bubble from="studio">
            Changed 8.1 so notice must be in writing, and left the notice period as it was.
          </Bubble>
          <ChangeCard summary="Changed 1 paragraph" />
        </Conversation>
      }
      details={<DocumentDetails outline={doc.model?.outline ?? []} selected={selected} />}
    />
  );
}

/**
 * What is in the document, from the document itself.
 *
 * The outline comes from the file's own headings rather than being written here, so it
 * cannot drift from what the User is looking at.
 */
function DocumentDetails(props: {
  outline: { text: string; level: number }[];
  selected?: number;
}) {
  return (
    <div style={{ padding: "14px 16px", display: "grid", gap: 14 }}>
      <div>
        <div
          style={{
            fontSize: 10.5,
            textTransform: "uppercase",
            letterSpacing: 0.6,
            color: "var(--muted)",
            marginBottom: 8,
          }}
        >
          {t("doc.in_this_document")}
        </div>
        {props.outline.length === 0 ? (
          <div style={{ fontSize: 12, color: "var(--muted)" }}>{t("doc.no_headings")}</div>
        ) : (
          <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "grid", gap: 6 }}>
            {props.outline.map((entry, index) => (
              <li
                key={`${entry.text}-${index}`}
                style={{
                  fontSize: 12,
                  paddingLeft: (entry.level - 1) * 12,
                  color: "var(--fg)",
                }}
              >
                {entry.text}
              </li>
            ))}
          </ul>
        )}
      </div>
      {props.selected !== undefined && (
        <div style={{ fontSize: 12, color: "var(--muted)" }}>{t("doc.selected_block")}</div>
      )}
    </div>
  );
}

export function SpreadsheetWorkspace(props: WorkspaceProps & { path?: string }) {
  const sheet = useSheet(props.path);

  const canvas = sheet.loading ? (
    <div style={{ width: 420, alignSelf: "flex-start" }}>
      <Progress steps={["Opening your spreadsheet", "Reading the sheets"]} current={1} />
    </div>
  ) : sheet.problem ? (
    <div style={{ width: 480, alignSelf: "flex-start" }}>
      <Failure kind="userActionable" headline={sheet.problem} action="Choose another file" />
    </div>
  ) : sheet.model ? (
    <SheetGrid model={sheet.model} note={t("doc.recalc_here")} />
  ) : null;

  return (
    <Workspace
      {...props}
      fileName={sheet.model?.fileName ?? "Q3 revenue model.xlsx"}
      toolbar={
        <>
          <Button small>Format</Button>
          <Button small>Chart</Button>
          <Button small>Pivot</Button>
          <Button small>Rules</Button>
        </>
      }
      status={
        <span style={{ fontSize: 11.5, color: "var(--live-fg)", fontWeight: 600 }}>
          1 change by me
        </span>
      }
      canvas={canvas}
      // The grid is the surface, not a page on a desk, so it takes the whole area and the
      // sheet names sit on its bottom edge where a spreadsheet keeps them.
      fill={Boolean(sheet.model)}
      conversation={
        <Conversation>
          <Bubble from="you">Add a 12% growth case next to the base case</Bubble>
          <Bubble from="studio">
            Added column D with 12% applied to each month and extended the total row. The chart
            picked it up.
          </Bubble>
          <ChangeCard summary="Changed 5 cells and 1 chart" />
        </Conversation>
      }
      details={
        <div>
          <DetailsSection label={t("details.what_changed")}>
            <DetailsLine>
              <span style={{ color: "var(--live-fg)", fontWeight: 700 }}>•</span>
              <div>
                Column D added, total row extended{" "}
                <span style={{ color: "var(--faint)" }}>{t("details.by_me")}</span>
              </div>
            </DetailsLine>
            <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
              <Button small>{t("details.undo_mine")}</Button>
            </div>
          </DetailsSection>
          <DetailsSection label={t("details.versions")}>
            <DetailsLine>
              <div style={{ flex: 1 }}>Now — growth case added</div>
            </DetailsLine>
            <DetailsLine>
              <div style={{ flex: 1 }}>20 min ago — you edited D8</div>
              <Button small>{t("details.go_back")}</Button>
            </DetailsLine>
          </DetailsSection>
        </div>
      }
    />
  );
}

/* -------------------------------------------------------------------- deck */

const BARS: [string, number, string][] = [
  ["Kenya", 130, "#8fa8b8"],
  ["Nigeria", 92, "#a8bcc9"],
  ["Ghana", 61, "#c2d1da"],
  ["Rest", 38, "#d9e2e8"],
];

export function DeckWorkspace(
  props: WorkspaceProps & { state: DeckState; active: number; onActive: (index: number) => void },
) {
  const deck = props.state;
  const active = props.active;
  const setActive = props.onActive;
  const [selected, setSelected] = useState<
    { item: number; refersTo: string; position?: number } | undefined
  >(undefined);

  const slides = deck.model?.slides ?? [];
  const slide = slides[active];

  // A click on the drawing tells us which element it was and, because the Core recorded
  // what each one came from, what could actually be changed.
  const onSlideClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const node = (event.target as HTMLElement | null)?.closest("[data-item]");
    const raw = node?.getAttribute("data-item");
    if (raw === null || raw === undefined || !slide) {
      setSelected(undefined);
      return;
    }
    const item = Number(raw);
    const target = slide.targets[item];
    setSelected(
      target ? { item, refersTo: target.refers_to, position: target.position } : undefined,
    );
  };

  return (
    <Workspace
      {...props}
      fileName={deck.model?.fileName ?? "Deck"}
      toolbar={
        <>
          <Button small>Text</Button>
          <Button small>Shape</Button>
          <Button small>Chart</Button>
          <Button small>Colours</Button>
        </>
      }
      status={
        <span style={{ fontSize: 11.5, color: "var(--muted)", fontWeight: 600 }}>
          {deck.problem
            ? deck.problem
            : selected
              ? t("deck.selected_shape")
              : `${slides.length} ${t("deck.slides")}`}
        </span>
      }
      canvas={
        deck.problem ? (
          <div style={{ padding: 24, color: "var(--muted)", maxWidth: 460 }}>{deck.problem}</div>
        ) : (
          <div style={{ display: "grid", gap: 12, justifyItems: "center" }}>
            <div
              onClick={onSlideClick}
              style={{
                background: "#fff",
                border: "1px solid #dedad2",
                boxShadow: "0 1px 5px rgba(0,0,0,.06)",
                width: 520,
                alignSelf: "center",
              }}
            >
              {slide ? (
                <div
                  style={{ width: "100%" }}
                  // Drawn by the Core from the User's own file, with every element
                  // carrying what it refers to.
                  dangerouslySetInnerHTML={{
                    __html: slide.svg.replace("<svg", '<svg style="width:100%;height:auto"'),
                  }}
                />
              ) : (
                <div style={{ padding: 24, color: "var(--muted)" }}>{t("common.loading")}</div>
              )}
            </div>
            {slides.length > 1 && (
              <div style={{ display: "flex", gap: 8 }} role="tablist">
                {slides.map((each, index) => (
                  <button
                    key={each.number}
                    role="tab"
                    aria-selected={index === active}
                    aria-label={each.title}
                    onClick={() => {
                      setActive(index);
                      setSelected(undefined);
                    }}
                    style={{
                      width: 54,
                      height: 32,
                      background: "#fff",
                      border:
                        index === active ? "2px solid var(--accent)" : "1px solid #dedad2",
                      borderRadius: 2,
                      fontSize: 10,
                      color: "var(--muted)",
                      cursor: "pointer",
                    }}
                  >
                    {each.number}
                  </button>
                ))}
              </div>
            )}
          </div>
        )
      }
      conversation={
        <Conversation>
          <Bubble from="you">Make a board deck from last quarter's numbers</Bubble>
          <Bubble from="studio">
            Built 8 slides from <b>Q3 revenue model.xlsx</b> — revenue, margin, runway and the
            regional split.
          </Bubble>
          <Bubble from="you">Regional split on its own slide with a chart</Bubble>
          <Bubble from="studio">
            Done — slide 5. I also fixed the contrast on the axis labels so it reads on a projector.
          </Bubble>
          <ChangeCard summary="Changed 3 slides" />
        </Conversation>
      }
      details={<DeckDetails />}
    />
  );
}

function DeckDetails() {
  return (
    <div>
      <DetailsSection label={t("details.what_changed")}>
        <DetailsLine>
          <span style={{ color: "var(--live-fg)", fontWeight: 700 }}>•</span>
          <div>
            Slide 5 — chart added and resized{" "}
            <span style={{ color: "var(--faint)" }}>{t("details.by_me")}</span>
          </div>
        </DetailsLine>
        <DetailsLine>
          <span style={{ color: "var(--live-fg)", fontWeight: 700 }}>•</span>
          <div>
            Slides 2–4 — figures updated{" "}
            <span style={{ color: "var(--faint)" }}>{t("details.by_me")}</span>
          </div>
        </DetailsLine>
        <DetailsLine>
          <span style={{ color: "var(--ink-soft)", fontWeight: 700 }}>•</span>
          <div>
            Slide 5 title reworded{" "}
            <span style={{ color: "var(--faint)" }}>{t("details.by_you")}</span>
          </div>
        </DetailsLine>
        <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
          <Button small>{t("details.undo_mine")}</Button>
          <Button small>{t("details.review_all")}</Button>
        </div>
      </DetailsSection>
      <DetailsSection label={t("details.where_from")}>
        <DetailsLine>
          <Icon name="sheet" size={13} stroke="var(--muted)" width={1.8} />
          <div>Q3 revenue model.xlsx — summary tab</div>
        </DetailsLine>
        <DetailsLine>
          <Icon name="document" size={13} stroke="var(--muted)" width={1.8} />
          <div>Last quarter's board pack</div>
        </DetailsLine>
      </DetailsSection>
      <DetailsSection label={t("details.worth_knowing")}>
        <DetailsLine>
          <Icon name="warning" size={13} stroke="var(--warn-fg)" />
          <div>I lightened the axis labels — the originals wouldn't read on a projector.</div>
        </DetailsLine>
      </DetailsSection>
      <DetailsSection label={t("details.versions")}>
        <DetailsLine>
          <div style={{ flex: 1 }}>Now — regional split added</div>
        </DetailsLine>
        <DetailsLine>
          <div style={{ flex: 1 }}>21:04 — you reworded slide 5</div>
          <Button small>{t("details.go_back")}</Button>
        </DetailsLine>
        <DetailsLine>
          <div style={{ flex: 1 }}>20:38 — first draft, 8 slides</div>
          <Button small>{t("details.go_back")}</Button>
        </DetailsLine>
      </DetailsSection>
    </div>
  );
}

/* --------------------------------------------------------- honest limits */

export function HonestLimits(props: WorkspaceProps) {
  return (
    <Workspace
      {...props}
      fileName="Partnership agreement — draft 3.docx"
      toolbar={<Button small>Comment</Button>}
      canvas={
        <div style={{ width: 560, alignSelf: "flex-start" }}>
          <div
            style={{
              background: "var(--warn-bg)",
              border: "1px solid #e6cE95",
              borderRadius: 10,
              padding: "13px 15px",
              display: "flex",
              gap: 11,
            }}
          >
            <Icon name="warning" size={17} stroke="var(--warn-fg)" />
            <div>
              <div style={{ fontWeight: 650, fontSize: 13.5, color: "#6f5210" }}>
                {t("limits.title")}
              </div>
              <p
                style={{
                  fontSize: 12.5,
                  lineHeight: 1.6,
                  color: "#5f4a15",
                  margin: "6px 0 0",
                }}
              >
                This document has 23 tracked changes and 14 comments. I can read them, but I can't
                write them back yet — so saving over this file would strip them out.
              </p>
            </div>
          </div>

          <Card style={{ marginTop: 14 }}>
            <div className="title">{t("limits.work_on_copy")}</div>
            <div className="sub">
              I'll make <b>Partnership agreement — draft 4.docx</b> with your changes, and leave
              draft 3 exactly as it is with all the markup.
            </div>
            <div style={{ display: "flex", gap: 9, marginTop: 14 }}>
              <Button primary>{t("limits.work_on_copy")}</Button>
              <Button>{t("limits.just_tell_me")}</Button>
            </div>
          </Card>
          <p className="hint" style={{ marginTop: 14 }}>
            {t("limits.i_check")}
          </p>
        </div>
      }
      conversation={
        <Conversation>
          <Bubble from="you">Tighten the termination clause and add a 60-day notice period</Bubble>
          <Bubble from="studio">
            Before I touch this — your file has <b>tracked changes and 14 comments</b> from your
            lawyer that I can't keep. If I edit this copy, they'd be lost.
          </Bubble>
        </Conversation>
      }
      details={
        <DetailsSection label={t("details.worth_knowing")}>
          <DetailsLine>
            <Icon name="warning" size={13} stroke="var(--warn-fg)" />
            <div>23 tracked changes and 14 comments cannot be written back.</div>
          </DetailsLine>
        </DetailsSection>
      }
    />
  );
}
