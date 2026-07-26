/**
 * Getting a document or a deck from the Core.
 *
 * Same shape as `useSheet`, and for the same reason: the Core reads the file and the
 * renderer draws what it is given. Nothing here parses a document or a deck.
 *
 * When the Core is not there — a browser during review, or before the Shell has started
 * it — a small sample stands in, and `source` says which one you are looking at, so a
 * screenshot can never quietly be of fixture data.
 */

import { useEffect, useState } from "react";

export type Source = "core" | "sample";

/** A document, read into an editable view with an identifier per block. */
export interface DocumentModel {
  fileName: string;
  html: string;
  headerHtml?: string;
  footerHtml?: string;
  blockCount: number;
  outline: { text: string; level: number }[];
}

/** What a click on a drawn element refers to. */
export interface Target {
  refers_to: string;
  position?: number;
}

export interface SlideView {
  number: number;
  title: string;
  svg: string;
  itemCount: number;
  targets: (Target | null)[];
}

export interface DeckModel {
  fileName: string;
  slides: SlideView[];
  active: number;
}

export const SAMPLE_DOCUMENT: DocumentModel = {
  fileName: "Sample.docx",
  html:
    '<h1 data-p="0">8. Termination</h1>' +
    '<p data-p="1">Either party may terminate this agreement for material breach, ' +
    "on thirty days' written notice.</p>" +
    '<h1 data-p="2">9. Confidentiality</h1>',
  blockCount: 3,
  outline: [
    { text: "8. Termination", level: 1 },
    { text: "9. Confidentiality", level: 1 },
  ],
};

export const SAMPLE_DECK: DeckModel = {
  fileName: "Sample.pptx",
  active: 0,
  slides: [
    {
      number: 1,
      title: "Revenue by region",
      itemCount: 1,
      targets: [{ refers_to: "shape", position: 0 }],
      svg:
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1280 720" width="1280" height="720">' +
        '<rect width="1280" height="720" fill="#ffffff"/>' +
        '<g data-item="0" data-shape="0">' +
        '<text x="96" y="140" font-size="44" fill="#1a1814">Revenue by region</text>' +
        "</g></svg>",
    },
  ],
};

interface State<T> {
  model?: T;
  /** Already in the User's words when it comes from the Core. */
  problem?: string;
  loading: boolean;
  source: Source;
}

export type DocumentState = State<DocumentModel>;
export type DeckState = State<DeckModel>;

function isProblem(value: unknown): value is { problem: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { problem?: unknown }).problem === "string"
  );
}

function isDocument(value: unknown): value is DocumentModel {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<DocumentModel>;
  return typeof candidate.fileName === "string" && typeof candidate.html === "string";
}

function isDeck(value: unknown): value is DeckModel {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<DeckModel>;
  return typeof candidate.fileName === "string" && Array.isArray(candidate.slides);
}

/**
 * Ask the Core for an artefact, falling back to the sample when there is no Core.
 *
 * Shared by both kinds because the only difference is which question is asked and how the
 * answer is recognised.
 */
function useArtefact<T>(
  path: string | undefined,
  ask: ((path: string) => Promise<unknown>) | undefined,
  recognise: (value: unknown) => value is T,
  sample: T,
): State<T> {
  const [state, setState] = useState<State<T>>({
    loading: Boolean(path),
    source: "sample",
    model: path ? undefined : sample,
  });

  useEffect(() => {
    if (!path || !ask) {
      setState({ loading: false, source: "sample", model: sample });
      return;
    }

    let cancelled = false;
    setState((current) => ({ ...current, loading: true }));

    void (async () => {
      try {
        const answer: unknown = await ask(path);
        if (cancelled) return;
        if (recognise(answer)) {
          setState({ loading: false, source: "core", model: answer });
        } else if (isProblem(answer)) {
          setState({ loading: false, source: "core", problem: answer.problem });
        } else {
          setState({ loading: false, source: "sample", model: sample });
        }
      } catch {
        if (cancelled) return;
        // The Core went away mid-question. Show the sample rather than an error the
        // User can do nothing about.
        setState({ loading: false, source: "sample", model: sample });
      }
    })();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);

  return state;
}

export function useDocument(path: string | undefined): DocumentState {
  const bridge = typeof window === "undefined" ? undefined : window.studio;
  return useArtefact(path, bridge?.document, isDocument, SAMPLE_DOCUMENT);
}

export function useDeck(path: string | undefined): DeckState {
  const bridge = typeof window === "undefined" ? undefined : window.studio;
  return useArtefact(path, bridge?.deck, isDeck, SAMPLE_DECK);
}
