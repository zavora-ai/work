/**
 * The User's own things: their folder, their work, and what they have told Work Studio.
 *
 * Every one of these replaced invented data. The interface said "Folders here are real
 * folders on your Mac" over a list of fourteen files that did not exist, and "This is
 * everything I go on" over three notes nobody had written. Both claims are now true or the
 * screen says it could not read them; neither is met with a plausible-looking substitute.
 */

import { useCallback, useEffect, useState } from "react";

function bridge() {
  return typeof window === "undefined" ? undefined : window.studio;
}

/* ------------------------------------------------------------------ folder */

export interface Entry {
  name: string;
  path: string;
  /** "spreadsheet", "document", "deck", "pdf", or absent for something else. */
  kind?: string;
  isFolder: boolean;
  size?: number;
  /** Seconds since the epoch. */
  changed?: number;
  count?: number;
}

export interface FolderState {
  /** How the location reads, e.g. "Documents › Work Studio". */
  location?: string;
  entries: Entry[];
  loading: boolean;
  problem?: string;
}

export function useFolder(within?: string) {
  const [state, setState] = useState<FolderState>({ entries: [], loading: true });

  const reload = useCallback(async () => {
    const studio = bridge();
    if (!studio?.files) {
      setState({ entries: [], loading: false });
      return;
    }
    setState((s) => ({ ...s, loading: true }));
    try {
      const answer = (await studio.files(within)) as {
        location?: string;
        entries?: Entry[];
        problem?: string;
      };
      if (answer.problem) {
        setState({ entries: [], loading: false, problem: answer.problem });
        return;
      }
      setState({
        location: answer.location,
        entries: answer.entries ?? [],
        loading: false,
      });
    } catch {
      setState({ entries: [], loading: false, problem: "Work Studio could not read your folder." });
    }
  }, [within]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const newFolder = useCallback(
    async (name: string) => {
      await bridge()?.newFolder?.({ name, within });
      await reload();
    },
    [within, reload],
  );

  return { state, reload, newFolder };
}

/* -------------------------------------------------------------------- work */

export interface Thread {
  id: string;
  /** What the User asked for, which is how they recognise it. */
  purpose: string;
  file?: string;
  changed: number;
}

export function useThreads(changedAt?: number) {
  const [threads, setThreads] = useState<Thread[]>([]);

  const reload = useCallback(async () => {
    try {
      const answer = (await bridge()?.threads?.()) as { threads?: Thread[] } | undefined;
      setThreads(answer?.threads ?? []);
    } catch {
      setThreads([]);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload, changedAt]);

  return { threads, reload };
}

/* ---------------------------------------------------------------- steering */

export interface Note {
  id: string;
  note: string;
  /** Where it came from, in the User's terms. */
  provenance: string;
  /** The question to put to the User, when Work Studio worked this out for itself. */
  asks?: string;
  scope: string;
}

export interface SteeringState {
  notes: Note[];
  /** Noticed, and waiting to be agreed to. These influence nothing yet. */
  proposed: Note[];
  global: Note[];
  loading: boolean;
  problem?: string;
}

export function useSteering(thread?: string, changedAt?: number) {
  const [state, setState] = useState<SteeringState>({
    notes: [],
    proposed: [],
    global: [],
    loading: true,
  });

  const reload = useCallback(async () => {
    const studio = bridge();
    if (!studio?.steering) {
      setState({ notes: [], proposed: [], global: [], loading: false });
      return;
    }
    try {
      const answer = (await studio.steering(thread)) as {
        notes?: Note[];
        proposed?: Note[];
        global?: Note[];
        problem?: string;
      };
      setState({
        notes: answer.notes ?? [],
        proposed: answer.proposed ?? [],
        global: answer.global ?? [],
        loading: false,
        problem: answer.problem,
      });
    } catch {
      setState({
        notes: [],
        proposed: [],
        global: [],
        loading: false,
        problem: "Work Studio could not read your notes.",
      });
    }
  }, [thread]);

  useEffect(() => {
    void reload();
  }, [reload, changedAt]);

  const add = useCallback(
    async (note: string) => {
      await bridge()?.addNote?.({ note, thread });
      await reload();
    },
    [thread, reload],
  );

  /** Accept, reword, stop applying, or forget. */
  const act = useCallback(
    async (id: string, action: "accept" | "reword" | "stop" | "forget", text?: string) => {
      await bridge()?.noteAction?.({ id, action, text });
      await reload();
    },
    [reload],
  );

  return { state, reload, add, act };
}
