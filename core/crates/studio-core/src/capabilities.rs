//! What each specialist may reach, and which of those are on.
//!
//! Modelled on ADK-Rust's `McpServerManager`: a set of named connections, each a command with
//! arguments and an environment, each able to be turned off, each with a state the interface
//! can show. What differs is what "state" honestly means here.
//!
//! ADK-Rust's manager holds long-lived processes and health-checks them, so it can say
//! `Running`. Work Studio starts a connection for the work at hand and stops it after, so
//! there is usually nothing running to report. Claiming `Running` would be a lie told by a
//! status light, and claiming `Stopped` would read as a fault. So the states here are the ones
//! we can actually answer: whether the User has it on, and whether the thing it needs is
//! present on this computer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use studio_store::Store;

/// What can be said about a connection without pretending to have checked more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Readiness {
    /// On, and what it needs is here.
    Ready,
    /// On, but what it needs is not on this computer, so work needing it will fail.
    Missing,
    /// The User has turned it off. Nothing will use it.
    Off,
}

impl Readiness {
    /// How the interface says it. Short enough for a row.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Missing => "Not installed",
            Self::Off => "Off",
        }
    }
}

/// One connection, as the interface shows it.
///
/// No command path and no environment values: a path is not the User's business and a value
/// may be a credential. The names of the variables are shown, because a missing one is
/// something the User can act on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityView {
    pub id: String,
    pub label: String,
    pub readiness: Readiness,
    /// How the readiness reads, in the User's words.
    pub status: String,
    /// Which specialists may use it.
    pub agents: Vec<String>,
    /// Names only, never values.
    pub needs: Vec<String>,
    /// True for the ones Work Studio provisioned, which the User may turn off but not remove.
    pub built_in: bool,
}

/// A connection being added.
#[derive(Debug, Clone, Deserialize)]
pub struct NewCapability {
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Which specialists may use it. Empty means none, which is the safe default.
    #[serde(default)]
    pub agents: Vec<String>,
}

/// What a specialist needs to actually run a connection.
///
/// Only a build that can do the work reads this, so a build without the engine never
/// constructs one.
#[cfg_attr(not(any(test, feature = "adk")), allow(dead_code))]
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub id: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub struct Capabilities<'a> {
    store: &'a Store,
}

impl<'a> Capabilities<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Add a connection, or update one that is already there by the same name.
    pub fn add(&self, id: &str, new: &NewCapability, built_in: bool) -> Result<(), String> {
        let label = new.label.trim();
        if label.is_empty() {
            return Err("a connection needs a name".to_string());
        }
        if new.command.trim().is_empty() {
            return Err("a connection needs something to run".to_string());
        }
        for agent in &new.agents {
            if !is_known_agent(agent) {
                return Err(format!("there is no {agent} specialist"));
            }
        }

        self.store
            .conn()
            .execute(
                "INSERT INTO capabilities (id, label, command, args, env, enabled, built_in,
                                           added_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, unixepoch(), unixepoch())
                 ON CONFLICT(id) DO UPDATE SET
                     label = excluded.label,
                     command = excluded.command,
                     args = excluded.args,
                     env = excluded.env,
                     updated_at = unixepoch()",
                rusqlite::params![
                    id,
                    label,
                    new.command.trim(),
                    serde_json::to_string(&new.args).unwrap_or_else(|_| "[]".into()),
                    serde_json::to_string(&new.env).unwrap_or_else(|_| "{}".into()),
                    built_in as i64,
                ],
            )
            .map_err(|e| e.to_string())?;

        self.allocate(id, &new.agents)?;
        Ok(())
    }

    /// Say exactly which specialists may use a connection.
    ///
    /// Replaces rather than adds, so the interface can send the set it is showing and the
    /// stored permission always matches what the User just saw.
    pub fn allocate(&self, id: &str, agents: &[String]) -> Result<(), String> {
        for agent in agents {
            if !is_known_agent(agent) {
                return Err(format!("there is no {agent} specialist"));
            }
        }
        // Acting on a connection that is not there must fail rather than report success. The
        // DELETE and the INSERTs below both match zero rows for an unknown id, so without this
        // the caller is told the allocation was changed and nothing was.
        self.must_exist(id)?;
        let conn = self.store.conn();
        conn.execute(
            "DELETE FROM capability_agents WHERE capability_id = ?1",
            [id],
        )
        .map_err(|e| e.to_string())?;
        for agent in agents {
            conn.execute(
                "INSERT OR IGNORE INTO capability_agents (capability_id, agent) VALUES (?1, ?2)",
                rusqlite::params![id, agent],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Refuse quietly-wrong work: an id nobody has is an error, not a no-op.
    fn must_exist(&self, id: &str) -> Result<(), String> {
        let found: i64 = self
            .store
            .conn()
            .query_row(
                "SELECT count(*) FROM capabilities WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if found == 0 {
            return Err(format!("there is no {id} connection"));
        }
        Ok(())
    }

    /// Turn a connection on or off.
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let changed = self
            .store
            .conn()
            .execute(
                "UPDATE capabilities SET enabled = ?2, updated_at = unixepoch() WHERE id = ?1",
                rusqlite::params![id, enabled as i64],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("there is no such connection".to_string());
        }
        Ok(())
    }

    /// Remove a connection the User added.
    ///
    /// One Work Studio provisioned may be turned off but not removed: the product depends on
    /// it, and offering removal would let the User break their own spreadsheets in a way they
    /// could not undo from the interface.
    pub fn remove(&self, id: &str) -> Result<(), String> {
        let built_in: Option<i64> = self
            .store
            .conn()
            .query_row(
                "SELECT built_in FROM capabilities WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .ok();
        match built_in {
            None => Err("there is no such connection".to_string()),
            Some(1) => {
                Err("this one came with Work Studio. You can turn it off instead.".to_string())
            }
            Some(_) => {
                self.store
                    .conn()
                    .execute("DELETE FROM capabilities WHERE id = ?1", [id])
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
        }
    }

    /// Everything, as the interface shows it.
    pub fn list(&self) -> Result<Vec<CapabilityView>, String> {
        let conn = self.store.conn();
        let mut statement = conn
            .prepare(
                "SELECT id, label, command, env, enabled, built_in FROM capabilities
                 ORDER BY built_in DESC, label",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let label: String = row.get(1)?;
                let command: String = row.get(2)?;
                let env: String = row.get(3)?;
                let enabled: i64 = row.get(4)?;
                let built_in: i64 = row.get(5)?;
                Ok((id, label, command, env, enabled == 1, built_in == 1))
            })
            .map_err(|e| e.to_string())?;

        let mut found = Vec::new();
        for row in rows {
            let (id, label, command, env, enabled, built_in) = row.map_err(|e| e.to_string())?;
            let needs: BTreeMap<String, String> = serde_json::from_str(&env).unwrap_or_default();
            let readiness = if !enabled {
                Readiness::Off
            } else if command_present(&command) {
                Readiness::Ready
            } else {
                Readiness::Missing
            };
            found.push(CapabilityView {
                agents: self.agents_for(&id)?,
                id,
                label,
                readiness,
                status: readiness.label().to_string(),
                // Names only. A value here may be a credential.
                needs: needs.keys().cloned().collect(),
                built_in,
            });
        }
        Ok(found)
    }

    /// The connections a specialist may actually use: allocated to it, on, and present.
    #[cfg_attr(not(any(test, feature = "adk")), allow(dead_code))]
    pub fn for_agent(&self, agent: &str) -> Result<Vec<Resolved>, String> {
        let conn = self.store.conn();
        let mut statement = conn
            .prepare(
                "SELECT c.id, c.label, c.command, c.args, c.env
                 FROM capabilities c
                 JOIN capability_agents a ON a.capability_id = c.id
                 WHERE a.agent = ?1 AND c.enabled = 1
                 ORDER BY c.label",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([agent], |row| {
                let args: String = row.get(3)?;
                let env: String = row.get(4)?;
                Ok(Resolved {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    command: row.get(2)?,
                    args: serde_json::from_str(&args).unwrap_or_default(),
                    env: serde_json::from_str(&env).unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut usable = Vec::new();
        for row in rows {
            let resolved = row.map_err(|e| e.to_string())?;
            // A connection whose command is absent is not offered. The specialist would
            // otherwise be told it has an ability it cannot use, and fail mid-task.
            if command_present(&resolved.command) {
                usable.push(resolved);
            }
        }
        Ok(usable)
    }

    fn agents_for(&self, id: &str) -> Result<Vec<String>, String> {
        let conn = self.store.conn();
        let mut statement = conn
            .prepare("SELECT agent FROM capability_agents WHERE capability_id = ?1 ORDER BY agent")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
}

/// The specialists that exist. A connection cannot be allocated to anything else.
pub fn is_known_agent(agent: &str) -> bool {
    matches!(agent, "spreadsheet" | "document" | "presentation")
}

/// Whether the thing a connection needs is on this computer.
fn command_present(command: &str) -> bool {
    let path = std::path::Path::new(command);
    if path.is_absolute() || command.contains('/') {
        return path.is_file();
    }
    // A bare name is looked for the way a shell would.
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(command).is_file()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let mut store = Store::open_in_memory().unwrap();
        store.migrate().unwrap();
        store
    }

    fn spreadsheet_capability(command: &str) -> NewCapability {
        NewCapability {
            label: "Spreadsheets".into(),
            command: command.into(),
            args: vec![],
            env: BTreeMap::new(),
            agents: vec!["spreadsheet".into()],
        }
    }

    #[test]
    fn a_connection_is_added_and_listed_in_the_users_terms() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        capabilities
            .add("sheets", &spreadsheet_capability("/bin/sh"), true)
            .unwrap();

        let listed = capabilities.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].label, "Spreadsheets");
        assert_eq!(listed[0].readiness, Readiness::Ready, "/bin/sh is present");
        assert_eq!(listed[0].status, "Ready");
        assert_eq!(listed[0].agents, vec!["spreadsheet".to_string()]);
        assert!(listed[0].built_in);
    }

    /// A path is not the User's business, and an environment value may be a credential.
    #[test]
    fn neither_a_path_nor_a_secret_reaches_the_interface() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        let mut env = BTreeMap::new();
        env.insert(
            "OPENAI_API_KEY".to_string(),
            "sk-do-not-show-this".to_string(),
        );
        capabilities
            .add(
                "sheets",
                &NewCapability {
                    label: "Spreadsheets".into(),
                    command: "/very/private/path/excel-server".into(),
                    args: vec!["--secret-flag".into()],
                    env,
                    agents: vec![],
                },
                false,
            )
            .unwrap();

        let shown = serde_json::to_string(&capabilities.list().unwrap()).unwrap();
        assert!(
            !shown.contains("sk-do-not-show-this"),
            "a value leaked: {shown}"
        );
        assert!(
            !shown.contains("/very/private/path"),
            "a path leaked: {shown}"
        );
        assert!(
            shown.contains("OPENAI_API_KEY"),
            "the name should be shown, because a missing one is actionable"
        );
    }

    /// The absence of a grant is the safe reading.
    #[test]
    fn a_specialist_reaches_nothing_it_was_not_given() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        capabilities
            .add("sheets", &spreadsheet_capability("/bin/sh"), true)
            .unwrap();

        assert_eq!(capabilities.for_agent("spreadsheet").unwrap().len(), 1);
        assert!(
            capabilities.for_agent("document").unwrap().is_empty(),
            "a connection allocated to one specialist must not reach another"
        );
        assert!(capabilities.for_agent("presentation").unwrap().is_empty());
    }

    #[test]
    fn turning_one_off_takes_it_away_from_every_specialist() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        capabilities
            .add("sheets", &spreadsheet_capability("/bin/sh"), true)
            .unwrap();
        capabilities.set_enabled("sheets", false).unwrap();

        assert!(capabilities.for_agent("spreadsheet").unwrap().is_empty());
        assert_eq!(capabilities.list().unwrap()[0].readiness, Readiness::Off);
        assert_eq!(capabilities.list().unwrap()[0].status, "Off");

        capabilities.set_enabled("sheets", true).unwrap();
        assert_eq!(capabilities.for_agent("spreadsheet").unwrap().len(), 1);
    }

    /// Saying a connection is ready when the thing it needs is absent would send a specialist
    /// to fail mid-task.
    #[test]
    fn something_not_installed_says_so_and_is_not_offered() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        capabilities
            .add(
                "sheets",
                &spreadsheet_capability("/nowhere/at/all/excel-server"),
                true,
            )
            .unwrap();

        assert_eq!(
            capabilities.list().unwrap()[0].readiness,
            Readiness::Missing
        );
        assert_eq!(capabilities.list().unwrap()[0].status, "Not installed");
        assert!(
            capabilities.for_agent("spreadsheet").unwrap().is_empty(),
            "a specialist must not be told it has an ability it cannot use"
        );
    }

    #[test]
    fn allocation_replaces_rather_than_accumulates() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        capabilities
            .add("sheets", &spreadsheet_capability("/bin/sh"), false)
            .unwrap();
        capabilities
            .allocate("sheets", &["document".into(), "presentation".into()])
            .unwrap();

        let listed = capabilities.list().unwrap();
        assert_eq!(
            listed[0].agents,
            vec!["document".to_string(), "presentation".to_string()],
            "the stored permission must match the set the User was shown"
        );
        assert!(capabilities.for_agent("spreadsheet").unwrap().is_empty());
    }

    #[test]
    fn what_came_with_work_studio_can_be_turned_off_but_not_removed() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        capabilities
            .add("sheets", &spreadsheet_capability("/bin/sh"), true)
            .unwrap();
        capabilities
            .add("mine", &spreadsheet_capability("/bin/sh"), false)
            .unwrap();

        let refused = capabilities.remove("sheets").unwrap_err();
        assert!(
            refused.contains("turn it off"),
            "it must say what to do instead: {refused}"
        );
        assert!(capabilities.remove("mine").is_ok());
        assert_eq!(capabilities.list().unwrap().len(), 1);
    }

    #[test]
    fn a_specialist_that_does_not_exist_is_refused() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        let mut bad = spreadsheet_capability("/bin/sh");
        bad.agents = vec!["accountant".into()];
        let error = capabilities.add("x", &bad, false).unwrap_err();
        assert!(error.contains("no accountant specialist"), "{error}");
        assert!(
            capabilities.list().unwrap().is_empty(),
            "nothing partial should remain"
        );
    }

    #[test]
    fn a_connection_needs_a_name_and_something_to_run() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        let mut nameless = spreadsheet_capability("/bin/sh");
        nameless.label = "  ".into();
        assert!(capabilities.add("x", &nameless, false).is_err());

        let mut nothing_to_run = spreadsheet_capability("");
        nothing_to_run.label = "Something".into();
        assert!(capabilities.add("y", &nothing_to_run, false).is_err());
    }

    #[test]
    fn removing_or_disabling_something_absent_says_so() {
        let store = store();
        let capabilities = Capabilities::new(&store);
        assert!(capabilities.remove("ghost").is_err());
        assert!(capabilities.set_enabled("ghost", false).is_err());
    }

    #[test]
    fn a_bare_command_is_found_the_way_a_shell_would() {
        assert!(command_present("sh"), "sh is on the PATH");
        assert!(!command_present("definitely-not-a-real-command-xyz"));
        assert!(command_present("/bin/sh"));
        assert!(!command_present("/bin/definitely-not-here"));
    }

    /// The reason this exists: `allocate` on an unknown id matched zero rows in every statement
    /// and returned Ok, so the interface was told a change had been made that had not.
    #[test]
    fn acting_on_a_connection_that_is_not_there_is_refused() {
        let mut store = Store::open_in_memory().unwrap();
        store.migrate().unwrap();
        let capabilities = Capabilities::new(&store);

        let error = capabilities
            .allocate("spreadsheet", &["spreadsheet".to_string()])
            .expect_err("a singular id nobody has must not report success");
        assert!(error.contains("no spreadsheet connection"), "{error}");
    }
}
