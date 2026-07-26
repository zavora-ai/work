/**
 * Getting a spreadsheet from the Core.
 *
 * The renderer asks; the Core reads the file with `zavora-xlsx`, formats every value
 * and answers with a `GridModel`. Nothing here parses a spreadsheet, because a
 * spreadsheet must have exactly one calculator.
 *
 * When the Core is not there — a browser during review, or before the Shell has
 * started it — the sample grid stands in. That fallback is deliberate and visible:
 * `source` says which one you are looking at, so a screenshot can never quietly be of
 * fixture data.
 */

import { useEffect, useState } from "react";

import { SAMPLE_GRID, type GridModel } from "../shared/grid.ts";

export type Source = "core" | "sample";

export interface SheetState {
  model?: GridModel;
  /** Already in the User's words when it comes from the Core. */
  problem?: string;
  loading: boolean;
  source: Source;
}

function isGridModel(value: unknown): value is GridModel {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<GridModel>;
  return typeof candidate.fileName === "string" && Array.isArray(candidate.sheets);
}

function isProblem(value: unknown): value is { problem: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { problem?: unknown }).problem === "string"
  );
}

export function useSheet(path: string | undefined): SheetState {
  const [state, setState] = useState<SheetState>({
    loading: Boolean(path),
    source: "sample",
    model: path ? undefined : SAMPLE_GRID,
  });

  useEffect(() => {
    if (!path) {
      setState({ loading: false, source: "sample", model: SAMPLE_GRID });
      return;
    }

    let cancelled = false;
    const bridge = typeof window === "undefined" ? undefined : window.studio;

    if (!bridge?.sheet) {
      // No Core to ask. Say so by falling back rather than showing an error the User
      // can do nothing about.
      setState({ loading: false, source: "sample", model: SAMPLE_GRID });
      return;
    }

    setState((current) => ({ ...current, loading: true }));

    void (async () => {
      try {
        const answer: unknown = await bridge.sheet(path);
        if (cancelled) return;
        if (isGridModel(answer)) {
          setState({ loading: false, source: "core", model: answer });
        } else if (isProblem(answer)) {
          const { problem } = answer;
          setState({ loading: false, source: "core", problem });
        } else {
          setState({
            loading: false,
            source: "core",
            problem: "that file could not be opened",
          });
        }
      } catch {
        if (cancelled) return;
        setState({
          loading: false,
          source: "core",
          problem: "that file could not be opened",
        });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [path]);

  return state;
}
