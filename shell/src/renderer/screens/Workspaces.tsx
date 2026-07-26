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

import React, { useEffect, useState } from "react";

import { useSheet } from "../useSheet.ts";
import type { DeckState, DocumentState } from "../useArtefact.ts";
import { useAsk, type Turn } from "../useAsk.ts";
import { useSteering } from "../useOwn.ts";
import { SteeringPanel } from "../components/SteeringPanel.tsx";
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

export function DocumentWorkspace(
  props: WorkspaceProps & {
    state: DocumentState;
    path?: string;
    thread?: string;
    /** Tell the app the file changed, so what is drawn is refetched. */
    onChanged?: () => void;
    /** A request already made in words, sent once when the file opens. */
    askOnOpen?: string;
  },
) {
  const doc = props.state;
  const thread = props.thread ?? "document";
  const [selected, setSelected] = useState<number | undefined>(undefined);
  const conversation = useAsk(props.path, thread, props.askOnOpen);
  const [editedAt, setEditedAt] = useState(0);
  const steering = useSteering(thread, conversation.state.answeredAt + editedAt);

  // When the file changed — by asking or by hand — what is drawn must be refetched.
  useEffect(() => {
    if (conversation.state.changedAt || editedAt) props.onChanged?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conversation.state.changedAt, editedAt]);

  /**
   * Rewrite a paragraph by hand.
   *
   * The same door an agent's change goes through, so one history holds both. The block
   * identifier the Core marked is what makes it possible to say which paragraph.
   */
  const rewrite = (block: number, text: string) => {
    if (!props.path) return;
    void (async () => {
      await window.studio?.edit?.({
        path: props.path!,
        sheet: String(block),
        cell: "paragraph",
        value: text,
        thread,
      });
      setEditedAt(Date.now());
    })();
  };

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
                // Typing here is the User's own change. The Core produced this markup from
                // their file and marked every block, so a rewrite can say which paragraph it
                // was without matching on the words.
                contentEditable={Boolean(props.path)}
                suppressContentEditableWarning
                onBlur={(event) => {
                  const block = (event.target as HTMLElement).closest("[data-p]");
                  const raw = block?.getAttribute("data-p");
                  const text = block?.textContent ?? "";
                  if (raw !== null && raw !== undefined) rewrite(Number(raw), text);
                }}
                dangerouslySetInnerHTML={{ __html: doc.model.html }}
              />
            ) : (
              <div style={{ color: "var(--muted)" }}>{t("common.loading")}</div>
            )}
          </div>
        )
      }
      conversation={
        <LiveConversation
          turns={conversation.state.turns}
          refused={conversation.state.refused}
          problem={conversation.state.problem}
          working={conversation.state.working}
          progress={conversation.state.progress}
          onAsk={conversation.ask}
        />
      }
      details={
        <div style={{ padding: "14px 16px", display: "grid", gap: 18 }}>
          <DocumentDetails outline={doc.model?.outline ?? []} selected={selected} />
          <SteeringPanel
            notes={steering.state.notes}
            proposed={steering.state.proposed}
            problem={steering.state.problem}
            onAdd={(note) => void steering.add(note)}
            onAct={(id, action, text) => void steering.act(id, action, text)}
          />
        </div>
      }
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

export function SpreadsheetWorkspace(
  props: WorkspaceProps & { path?: string; thread?: string; askOnOpen?: string },
) {
  const conversation = useAsk(props.path, props.thread ?? "spreadsheet", props.askOnOpen);
  // What the grid says is selected. Formatting belongs to a selection, so the toolbar cannot
  // act until it knows one.
  const [selection, setSelection] = useState<{ sheet: string; range: string } | undefined>();

  const undoLast = async () => {
    if (!props.path) return;
    await window.studio?.undo?.({ path: props.path, thread: props.thread ?? "spreadsheet" });
    setEditedAt(Date.now());
  };

  const applyFormat = async (how: Record<string, unknown>) => {
    if (!props.path || !selection) return;
    await window.studio?.format?.({
      path: props.path,
      sheet: selection.sheet,
      range: selection.range,
      how,
      thread: props.thread ?? "spreadsheet",
    });
    setEditedAt(Date.now());
  };
  // What Work Studio goes on, refetched after every change so an accepted note shows at once.
  // Keyed on every exchange, not just a file change: being told a preference changes no file.
  const steering = useSteering(
    props.thread ?? "spreadsheet",
    conversation.state.answeredAt,
  );
  // Bumped when the User changes a cell themselves.
  const [editedAt, setEditedAt] = useState(0);
  // Reload the grid when Work Studio has changed the file.
  const sheet = useSheet(props.path, Math.max(conversation.state.changedAt, editedAt));

  const canvas = sheet.loading ? (
    <div style={{ width: 420, alignSelf: "flex-start" }}>
      <Progress steps={["Opening your spreadsheet", "Reading the sheets"]} current={1} />
    </div>
  ) : sheet.problem ? (
    <div style={{ width: 480, alignSelf: "flex-start" }}>
      <Failure kind="userActionable" headline={sheet.problem} action="Choose another file" />
    </div>
  ) : sheet.model ? (
    <SheetGrid
      model={sheet.model}
      note={t("doc.recalc_here")}
      onEdit={
        props.path
          ? (sheetName, cell, value) => {
              void (async () => {
                await window.studio?.edit?.({
                  path: props.path!,
                  sheet: sheetName,
                  cell,
                  value,
                  thread: props.thread ?? "spreadsheet",
                });
                // Reload so the number on screen is the number in the file.
                setEditedAt(Date.now());
              })();
            }
          : undefined
      }
      onSelection={(sheet, range) => setSelection({ sheet, range })}
      onUndo={props.path ? () => void undoLast() : undefined}
      onEditMany={
        props.path
          ? (sheetName, cells) => {
              void (async () => {
                // One write for the whole block: pasting twelve cells is one action, not twelve.
                const [first, ...rest] = cells;
                if (!first) return;
                await window.studio?.edit?.({
                  path: props.path!,
                  sheet: sheetName,
                  cell: first.cell,
                  value: first.value,
                  more: rest,
                  thread: props.thread ?? "spreadsheet",
                });
                setEditedAt(Date.now());
              })();
            }
          : undefined
      }
    />
  ) : null;

  return (
    <Workspace
      {...props}
      fileName={sheet.model?.fileName ?? "Q3 revenue model.xlsx"}
      toolbar={
        <>
          {/* These read Format, Chart, Pivot and Rules and did nothing at all. What is here
              now acts on whatever is selected, through the same gate and history as every
              other change. Chart and Pivot are not here because they are not built: an
              absent control is better than one that lies. */}
          <Button small title="Undo the last change" onClick={() => void undoLast()}>
            Undo
          </Button>
          <Button small title="Bold" onClick={() => void applyFormat({ bold: true })}>
            B
          </Button>
          <Button small title="Italic" onClick={() => void applyFormat({ italic: true })}>
            I
          </Button>
          <Button
            small
            title="Show as money"
            onClick={() => void applyFormat({ number_format: "#,##0.00" })}
          >
            0.00
          </Button>
          <Button
            small
            title="Show as a percentage"
            onClick={() => void applyFormat({ number_format: "0.0%" })}
          >
            %
          </Button>
          <Button
            small
            title="Shade it"
            onClick={() => void applyFormat({ background_color: "#FFF3C4" })}
          >
            Shade
          </Button>
          <Button
            small
            title="Plain again"
            onClick={() =>
              void applyFormat({
                bold: false,
                italic: false,
                number_format: "General",
                background_color: "#FFFFFF",
              })
            }
          >
            Plain
          </Button>
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
        <LiveConversation
          turns={conversation.state.turns}
          refused={conversation.state.refused}
          problem={conversation.state.problem}
          working={conversation.state.working}
          progress={conversation.state.progress}
          onAsk={conversation.ask}
        />
      }
      details={
        <div style={{ padding: "14px 16px" }}>
          <SteeringPanel
            notes={steering.state.notes}
            proposed={steering.state.proposed}
            problem={steering.state.problem}
            onAdd={(note) => void steering.add(note)}
            onAct={(id, action, text) => void steering.act(id, action, text)}
          />
        </div>
      }
    />
  );
}

/**
 * The conversation, from what was actually said.
 *
 * Falls back to nothing rather than to invented dialogue: a panel showing a conversation
 * that never happened is worse than an empty one, because the User cannot tell which is
 * which.
 */
function LiveConversation(props: {
  turns: Turn[];
  refused: string[];
  problem?: string;
  working: boolean;
  progress?: string;
  onAsk: (asked: string) => void;
}) {
  return (
    <Conversation onAsk={props.onAsk} working={props.working} progress={props.progress}>
      {props.turns.map((turn, index) => (
        <Bubble key={index} from={turn.from}>
          {turn.text}
        </Bubble>
      ))}
      {props.problem ? <Bubble from="studio">{props.problem}</Bubble> : null}
      {props.refused.length > 0 ? (
        <Card>
          <div style={{ fontSize: 11.5, fontWeight: 650, marginBottom: 4 }}>
            {t("kickoff.not_done")}
          </div>
          {props.refused.map((line, index) => (
            <div key={index} style={{ fontSize: 11.5, color: "var(--muted)" }}>
              {line}
            </div>
          ))}
        </Card>
      ) : null}
    </Conversation>
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
  props: WorkspaceProps & {
    state: DeckState;
    active: number;
    onActive: (index: number) => void;
    path?: string;
    thread?: string;
    onChanged?: () => void;
    /** A request already made in words, sent once when the file opens. */
    askOnOpen?: string;
  },
) {
  const deck = props.state;
  const thread = props.thread ?? "deck";
  const conversation = useAsk(props.path, thread, props.askOnOpen);
  const steering = useSteering(thread, conversation.state.answeredAt);

  useEffect(() => {
    if (conversation.state.changedAt) props.onChanged?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conversation.state.changedAt]);
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
        <LiveConversation
          turns={conversation.state.turns}
          refused={conversation.state.refused}
          problem={conversation.state.problem}
          working={conversation.state.working}
          progress={conversation.state.progress}
          onAsk={conversation.ask}
        />
      }
      details={
        <div style={{ padding: "14px 16px", display: "grid", gap: 18 }}>
          {/* What a click resolved to, so the User can see the thing they are about to
              change rather than guessing from a highlight. */}
          {selected ? (
            <p style={{ fontSize: 12, color: "var(--muted)", margin: 0 }}>
              {t("deck.selected_shape")}
            </p>
          ) : null}
          <SteeringPanel
            notes={steering.state.notes}
            proposed={steering.state.proposed}
            problem={steering.state.problem}
            onAdd={(note) => void steering.add(note)}
            onAct={(id, action, text) => void steering.act(id, action, text)}
          />
        </div>
      }
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
