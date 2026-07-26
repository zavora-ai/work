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
