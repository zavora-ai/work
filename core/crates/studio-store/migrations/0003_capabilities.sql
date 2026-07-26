-- What each specialist may reach, and which of those the User has turned on.
--
-- Separate from `connectors`, which holds accounts the User has signed into — Gmail and the
-- like. This holds the capability connections a specialist works through, the things that
-- give it the ability to open a spreadsheet or lay out a slide.
--
-- Modelled on ADK-Rust's `McpServerConfig`: a command, its arguments, its environment, and
-- whether it is disabled. The one field deliberately not carried across is `auto_approve`.
-- ADK-Rust keeps it for compatibility with `mcp.json` files and states that it does not
-- bypass authorisation; here it would be a field that looks like it grants permission while
-- the side-effect gate ignores it, so it is better absent than misleading (Property 2).

CREATE TABLE IF NOT EXISTS capabilities (
  id          TEXT PRIMARY KEY,
  -- What the User sees. Never a path.
  label       TEXT NOT NULL,
  command     TEXT NOT NULL,
  args        TEXT NOT NULL DEFAULT '[]',
  -- Names and values of environment variables the command needs.
  --
  -- A value here may be a credential, so nothing in this column is ever put into a payload
  -- bound for the interface: the Settings view lists the names and never the values.
  env         TEXT NOT NULL DEFAULT '{}',
  enabled     INTEGER NOT NULL DEFAULT 1,
  -- Set when Work Studio provisioned it rather than the User adding it, so the interface can
  -- decline to offer removal of something the product depends on.
  built_in    INTEGER NOT NULL DEFAULT 0,
  added_at    INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

-- Which specialists may use which capability.
--
-- A row is permission. No row means the specialist cannot reach it, which is why this is a
-- table of grants rather than a column of exclusions: the absence of a grant is the safe
-- reading, and a new specialist starts with nothing.
CREATE TABLE IF NOT EXISTS capability_agents (
  capability_id TEXT NOT NULL REFERENCES capabilities(id) ON DELETE CASCADE,
  agent         TEXT NOT NULL CHECK (agent IN ('spreadsheet', 'document', 'presentation')),
  PRIMARY KEY (capability_id, agent)
);

CREATE INDEX IF NOT EXISTS capability_agents_by_agent ON capability_agents (agent);
