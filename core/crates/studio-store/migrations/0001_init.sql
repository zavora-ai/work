-- Zavora Work Studio — initial schema.
-- Mirrors the data model in .kiro/specs/zavora-work-studio/design.md.
-- Artefacts themselves are ordinary files in the User's own folder; only
-- metadata lives here. Credentials never live here — they are in the OS keychain.

PRAGMA foreign_keys = ON;

CREATE TABLE jobs (
  id                TEXT PRIMARY KEY,
  kind              TEXT NOT NULL CHECK (kind IN ('scheduled','one_off')),
  template_id       TEXT,
  purpose           TEXT NOT NULL,
  state             TEXT NOT NULL CHECK (state IN
                      ('draft','awaiting_kickoff','live','paused',
                       'active','finished','needs_attention','retired')),
  schedule_kind     TEXT CHECK (schedule_kind IS NULL OR schedule_kind IN
                      ('time_of_day','weekdays','interval','manual')),
  schedule_spec     TEXT,
  schedule_cron     TEXT,
  timezone          TEXT NOT NULL,
  missed_run_policy TEXT CHECK (missed_run_policy IS NULL OR missed_run_policy IN
                      ('run_once_on_wake','skip_to_next')),
  out_tray_policy   TEXT NOT NULL DEFAULT 'always'
                      CHECK (out_tray_policy IN ('always','on_change')),
  read_only         INTEGER NOT NULL DEFAULT 0,
  output_folder     TEXT,
  retry_limit       INTEGER NOT NULL DEFAULT 3,
  next_run_at       INTEGER,
  last_run_id       TEXT,
  consecutive_failures INTEGER NOT NULL DEFAULT 0,
  created_at        INTEGER NOT NULL,
  updated_at        INTEGER NOT NULL,
  -- a one_off Job never carries a schedule
  CHECK (kind = 'scheduled' OR schedule_kind IS NULL)
);

CREATE TABLE job_runs (
  id            TEXT PRIMARY KEY,
  job_id        TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  mode          TEXT NOT NULL CHECK (mode IN ('kickoff_dry_run','live','manual')),
  started_at    INTEGER NOT NULL,
  finished_at   INTEGER,
  outcome       TEXT CHECK (outcome IS NULL OR outcome IN
                  ('completed','escalated','failed_transient','failed_user','suppressed')),
  summary       TEXT,
  spend_micros  INTEGER NOT NULL DEFAULT 0,
  failover_used INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX job_runs_by_job ON job_runs(job_id, started_at DESC);

-- One row per Job currently executing. The primary key is what makes run
-- exclusivity (Requirement 9.4) an database guarantee rather than a check-then-act.
CREATE TABLE job_leases (
  job_id       TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
  run_id       TEXT NOT NULL,
  acquired_at  INTEGER NOT NULL,
  heartbeat_at INTEGER NOT NULL
);

CREATE TABLE tray_items (
  id           TEXT PRIMARY KEY,
  job_id       TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  run_id       TEXT REFERENCES job_runs(id),
  class        TEXT NOT NULL CHECK (class IN ('kickoff','escalation','finding','attention')),
  headline     TEXT NOT NULL,
  detail       TEXT NOT NULL,
  -- The underlying cause, for consolidation. One expired account raises one item
  -- however many Jobs depend on it (Requirement 13.8). Matched by equality: an
  -- earlier attempt matched a substring of the prose, and SQLite's case-insensitive
  -- LIKE made the account "X" collide with the word "expired".
  cause        TEXT,
  payload_kind TEXT CHECK (payload_kind IS NULL OR payload_kind IN ('output','manifest')),
  payload      BLOB,
  choices      TEXT,
  created_at   INTEGER NOT NULL,
  resolved_at  INTEGER,
  resolution   TEXT CHECK (resolution IS NULL OR resolution IN
                 ('approved','approved_with_edits','approved_with_exclusions',
                  'approved_once','rejected','chosen','dismissed'))
);
CREATE INDEX tray_unresolved ON tray_items(resolved_at, created_at DESC);
CREATE INDEX tray_cause ON tray_items(class, cause) WHERE resolved_at IS NULL;

CREATE TABLE steering_notes (
  id         TEXT PRIMARY KEY,
  job_id     TEXT REFERENCES jobs(id) ON DELETE CASCADE, -- NULL = global note
  scope      TEXT NOT NULL DEFAULT 'job'
               CHECK (scope IN ('job','everything','document','deck','spreadsheet')),
  note       TEXT NOT NULL,
  origin     TEXT NOT NULL CHECK (origin IN
               ('explicit','rejection','derived_from_edit',
                'derived_from_exclusion','derived_from_choice')),
  confirmed  INTEGER NOT NULL DEFAULT 1,
  active     INTEGER NOT NULL DEFAULT 1,
  seq        INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  -- a global note is never scoped to 'job', and a per-Job note always is
  CHECK ((job_id IS NULL AND scope <> 'job') OR (job_id IS NOT NULL AND scope = 'job'))
);
CREATE INDEX steering_by_job ON steering_notes(job_id, seq);
CREATE INDEX steering_global ON steering_notes(scope, seq) WHERE job_id IS NULL;

CREATE TABLE artefacts (
  id              TEXT PRIMARY KEY,
  kind            TEXT NOT NULL CHECK (kind IN ('document','deck','spreadsheet','pdf')),
  file_path       TEXT NOT NULL UNIQUE,
  display_name    TEXT NOT NULL,
  derived_from    TEXT REFERENCES artefacts(id),
  last_author     TEXT NOT NULL CHECK (last_author IN ('user','studio')),
  last_editor_app TEXT,
  content_hash    TEXT NOT NULL,
  mtime           INTEGER NOT NULL,
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL
);

CREATE TABLE artefact_changes (
  artefact_id TEXT NOT NULL REFERENCES artefacts(id) ON DELETE CASCADE,
  seq         INTEGER NOT NULL,
  author      TEXT NOT NULL CHECK (author IN ('user','studio')),
  operation   TEXT NOT NULL,
  description TEXT NOT NULL,
  ts          INTEGER NOT NULL,
  PRIMARY KEY (artefact_id, seq)
);

-- Which Jobs have touched which Artefacts. The Repository's "Used in" column.
CREATE TABLE artefact_jobs (
  artefact_id TEXT NOT NULL REFERENCES artefacts(id) ON DELETE CASCADE,
  job_id      TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  first_ts    INTEGER NOT NULL,
  PRIMARY KEY (artefact_id, job_id)
);

CREATE TABLE deliveries (
  id            TEXT PRIMARY KEY,
  run_id        TEXT NOT NULL REFERENCES job_runs(id) ON DELETE CASCADE,
  connector     TEXT NOT NULL,
  action        TEXT NOT NULL,
  target        TEXT,
  external_ref  TEXT,
  reversibility TEXT NOT NULL CHECK (reversibility IN ('reversible','partial','irreversible')),
  reversal_note TEXT,
  reversal_expires_at INTEGER,
  reversed_at   INTEGER,
  ts            INTEGER NOT NULL,
  -- an irreversible delivery can never carry a reversal window
  CHECK (reversibility <> 'irreversible' OR reversal_expires_at IS NULL)
);

CREATE TABLE connectors (
  id           TEXT PRIMARY KEY,
  account      TEXT NOT NULL,
  scopes       TEXT NOT NULL,
  status       TEXT NOT NULL CHECK (status IN ('connected','expired','revoked','disconnected')),
  connected_at INTEGER NOT NULL,
  checked_at   INTEGER
);

-- Append-only. Enforced by triggers below, not by convention.
CREATE TABLE activity_log (
  seq      INTEGER PRIMARY KEY AUTOINCREMENT,
  ts       INTEGER NOT NULL,
  job_id   TEXT,
  run_id   TEXT,
  category TEXT NOT NULL CHECK (category IN
             ('action','failover','retry','recovered','connector','spend','privacy')),
  detail   TEXT NOT NULL
);

CREATE TRIGGER activity_log_no_update
BEFORE UPDATE ON activity_log
BEGIN
  SELECT RAISE(ABORT, 'activity_log is append-only');
END;

CREATE TRIGGER activity_log_no_delete
BEFORE DELETE ON activity_log
BEGIN
  SELECT RAISE(ABORT, 'activity_log is append-only');
END;

CREATE TABLE spend_ledger (
  id      TEXT PRIMARY KEY,
  ts      INTEGER NOT NULL,
  job_id  TEXT,
  surface TEXT NOT NULL CHECK (surface IN ('proactive','documents','internal')),
  tier    TEXT NOT NULL CHECK (tier IN ('fast','balanced','best')),
  micros  INTEGER NOT NULL
);
CREATE INDEX spend_by_day ON spend_ledger(ts);
