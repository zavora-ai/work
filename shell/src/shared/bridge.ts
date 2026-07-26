/**
 * The renderer's entire capability surface.
 *
 * One declaration, referenced by both sides: the preload implements it, the
 * renderer consumes it. Widening the renderer's power means adding a method here,
 * which shows up in a diff.
 */

export interface EventBatch {
  events: { seq: number; kind: unknown }[];
  /** The renderer's resume point is too old; refetch rather than replay. */
  refetchRequired: boolean;
  latestSeq: number;
}

/** What the Core says when it cannot open a file: already in the User's words. */
export interface Problem {
  problem: string;
}

export interface StudioBridge {
  /** Is the Core up? */
  health(): Promise<{ ready: boolean }>;
  /** Everything that happened after `since`. */
  events(since: number): Promise<EventBatch>;
  /**
   * The spreadsheet at `path`, read and formatted by the Core.
   *
   * The renderer never parses a spreadsheet itself, so there is only ever one
   * calculator and the number on screen is the number in the file.
   */
  /**
   * Ask the User for one of their files, and answer with its path.
   *
   * `undefined` if they changed their mind. The renderer never sees the filesystem,
   * only the one path the User chose.
   */
  openFile(): Promise<string | undefined>;

  /**
   * Ask Work Studio to change the open file.
   *
   * Answers when the work is finished. Progress arrives on the event stream in the
   * meantime, because the work takes long enough that silence would read as a hang.
   */
  /** What each specialist may reach, and whether it is on. */
  capabilities(): Promise<unknown>;
  addCapability(body: {
    label: string;
    command: string;
    args?: string[];
    env?: Record<string, string>;
    agents?: string[];
  }): Promise<unknown>;
  /** Turn one on or off, remove it, or say which specialists may use it. */
  capabilityAction(body: { id: string; action: string; agents?: string[] }): Promise<unknown>;

  /**
   * A change the User made by hand.
   *
   * Goes the same way an agent's change does, so one history holds both.
   */
  edit(body: {
    path: string;
    sheet: string;
    cell: string;
    value: string;
    thread?: string;
  }): Promise<unknown>;

  /** What is really in the User's folder. */
  files(within?: string): Promise<unknown>;
  newFolder(body: { name: string; within?: string }): Promise<unknown>;
  /** The pieces of work the User has done. */
  threads(): Promise<unknown>;
  /** One piece of work: what was said, and what Work Studio goes on. */
  thread(id: string): Promise<unknown>;
  steering(id?: string): Promise<unknown>;
  addNote(body: { note: string; thread?: string }): Promise<unknown>;
  /** Accept, reword, stop or forget a note. */
  noteAction(body: { id: string; action: string; text?: string }): Promise<unknown>;

  ask(request: { asked: string; path: string; thread?: string }): Promise<unknown>;

  sheet(path: string): Promise<unknown>;

  /**
   * The document at `path`, read by the Core into an editable view.
   *
   * The markup carries an identifier per block, which is what makes a change
   * attributable to the paragraph the User clicked.
   */
  document(path: string): Promise<unknown>;

  /**
   * The deck at `path`, drawn by the Core one slide at a time.
   *
   * Each drawn element carries what it refers to, so a click resolves to a shape
   * that can actually be changed.
   */
  deck(path: string): Promise<unknown>;
}
