-- What was said, so returning to a piece of work does not begin again.
--
-- The conversation was held in the interface's own memory, which meant switching screens
-- lost it and closing the application lost everything. A thread is a piece of work, so a
-- turn belongs to a Job and goes when that work does.

CREATE TABLE IF NOT EXISTS thread_turns (
  seq       INTEGER PRIMARY KEY AUTOINCREMENT,
  thread_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  said_by   TEXT NOT NULL CHECK (said_by IN ('you', 'studio')),
  text      TEXT NOT NULL,
  ts        INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS thread_turns_by_thread ON thread_turns (thread_id, seq);
