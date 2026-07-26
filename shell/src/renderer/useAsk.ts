/**
 * Asking for a change, and following it while it happens.
 *
 * The work takes the better part of a minute, so the wait is the design problem, not the
 * answer. Progress comes from the Core's own event stream — the same ordered, resumable
 * stream a reload replays — rather than a second channel invented for this.
 *
 * Nothing here decides anything. What was changed, what was refused and what may be undone
 * are all the Core's answers; this only shows them.
 */

import { useCallback, useEffect, useRef, useState } from "react";

export interface Turn {
  /** Who said it. */
  from: "you" | "studio";
  text: string;
}

export interface AskState {
  /** The conversation so far, oldest first. */
  turns: Turn[];
  /** What is happening now, in the User's words. Absent when nothing is. */
  progress?: string;
  /** True while the work is being done. */
  working: boolean;
  /** A problem, already in the User's words. */
  problem?: string;
  /** Operations Work Studio declined to perform, and why. Never hidden. */
  refused: string[];
  /** Bumped whenever the file on disk changed, so a view can reload. */
  changedAt: number;
}

interface Answer {
  said?: string;
  changed?: boolean;
  refused?: string[];
  problem?: string;
}

function isAnswer(value: unknown): value is Answer {
  return typeof value === "object" && value !== null;
}

/**
 * How often to look for progress.
 *
 * Half a second is often enough to feel live and rare enough to cost nothing, since the
 * Core answers from memory.
 */
const POLL_MS = 500;

export function useAsk(path: string | undefined, thread: string) {
  const [state, setState] = useState<AskState>({
    turns: [],
    working: false,
    refused: [],
    changedAt: 0,
  });
  // The resume point, so each poll asks only for what it has not seen.
  const since = useRef(0);

  // What was said before. Returning to a piece of work should not begin again: the turns are
  // kept by the Core, and without this the panel came back empty even though they were there.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const bridge = typeof window === "undefined" ? undefined : window.studio;
      if (!bridge?.thread) return;
      try {
        const answer = (await bridge.thread(thread)) as { turns?: Turn[] } | undefined;
        if (cancelled) return;
        const turns = answer?.turns ?? [];
        if (turns.length > 0) {
          setState((s) => (s.turns.length === 0 ? { ...s, turns } : s));
        }
      } catch {
        // Not being able to read the conversation is not worth an error the User can do
        // nothing about; the panel simply starts empty.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [thread]);

  const ask = useCallback(
    async (asked: string) => {
      const bridge = typeof window === "undefined" ? undefined : window.studio;
      if (!asked.trim()) return;
      if (!path) {
        setState((s) => ({ ...s, problem: "Open a file first, then ask for a change." }));
        return;
      }
      if (!bridge?.ask) {
        setState((s) => ({ ...s, problem: "Work Studio is not ready yet." }));
        return;
      }

      setState((s) => ({
        ...s,
        turns: [...s.turns, { from: "you", text: asked }],
        working: true,
        problem: undefined,
        refused: [],
        progress: undefined,
      }));

      // Follow the event stream while the work runs.
      const timer = window.setInterval(() => {
        void (async () => {
          try {
            const answer: unknown = await bridge.events?.(since.current);
            if (!answer || typeof answer !== "object") return;
            // The Core tags an event's kind on the value itself, and reports the
            // sequence to resume from. Both shapes are the Core's; the names here match
            // what it actually sends rather than what would be convenient.
            const { events, latestSeq } = answer as {
              events?: { kind?: { type?: string; message?: string } }[];
              latestSeq?: number;
            };
            if (typeof latestSeq === "number") since.current = latestSeq;
            const latest = (events ?? [])
              .filter((event) => event.kind?.type === "progress")
              .map((event) => event.kind?.message)
              .filter((message): message is string => typeof message === "string")
              .pop();
            if (latest) setState((s) => ({ ...s, progress: latest }));
          } catch {
            // A missed poll is not worth telling the User about; the next one will do.
          }
        })();
      }, POLL_MS);

      try {
        const answer: unknown = await bridge.ask({ asked, path, thread });
        if (!isAnswer(answer)) throw new Error("no answer");
        if (answer.problem) {
          setState((s) => ({
            ...s,
            working: false,
            progress: undefined,
            problem: answer.problem,
          }));
          return;
        }
        setState((s) => ({
          ...s,
          turns: [...s.turns, { from: "studio", text: answer.said ?? "" }],
          working: false,
          progress: undefined,
          refused: answer.refused ?? [],
          // Only bumped when the file really changed, so a view does not reload for
          // an answer that changed nothing.
          changedAt: answer.changed ? Date.now() : s.changedAt,
        }));
      } catch {
        setState((s) => ({
          ...s,
          working: false,
          progress: undefined,
          problem: "Work Studio could not finish that.",
        }));
      } finally {
        window.clearInterval(timer);
      }
    },
    [path, thread],
  );

  return { state, ask };
}
