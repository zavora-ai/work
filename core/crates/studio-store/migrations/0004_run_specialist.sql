-- Which specialist did the work, and how long the User waited.
--
-- job_runs recorded that a run happened but not who did it, so every per-specialist figure on
-- the Agents screen had to be invented — "Accepted as-is 72%", "Typical wait 4.1s". A figure
-- nobody measured is worse than a missing one: it looks like evidence.
--
-- Nullable on purpose. Runs already recorded genuinely do not know which specialist did them,
-- and backfilling a guess would manufacture the very data this exists to stop inventing.
ALTER TABLE job_runs ADD COLUMN specialist TEXT;

CREATE INDEX job_runs_by_specialist ON job_runs(specialist, started_at DESC);
