//! Zavora Work Studio — Core.
//!
//! A standalone binary. The Electron shell supervises it and talks to it over
//! authenticated loopback; nothing about this binary depends on Electron, so the
//! shell is replaceable without touching product logic.
//!
//! At this stage the Core proves the engine invariants and opens its store. The
//! loopback API arrives with task 1.5 and the ADK-Rust runner with task 3.5.

mod api;
mod capabilities;
mod keeper;

use studio_jobs::{Job, JobKind, JobState};
use studio_store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::var("ZWS_DATA_DIR").unwrap_or_else(|_| ".zws".to_string());
    std::fs::create_dir_all(&dir)?;
    let path = std::path::Path::new(&dir).join("studio.db");

    let store = Store::open(&path)?;
    store.log("privacy", "store opened on this device", None, None)?;

    println!(
        "Core ready · store at {} · migrations {:?}",
        path.display(),
        store.applied_migrations()?
    );

    // A one-off piece of work and a scheduled one, side by side in one list —
    // the unification the interface shows (Requirement 3.2).
    let deck = Job::new("j-deck", JobKind::OneOff, "Board deck from last quarter");
    let mut monitor = Job::new("j-health", JobKind::Scheduled, "Computer health").read_only(true);
    monitor.activate()?;

    println!(
        "  {} — {}\n  {} — {}",
        deck.purpose, deck.state, monitor.purpose, monitor.state
    );
    debug_assert_eq!(monitor.state, JobState::Live);

    // The loopback channel. The Shell supervises this process, holds the token,
    // and is the only thing that can reach the port (task 1.5).
    // The Shell mints the token and passes it at spawn. The Core never writes it
    // anywhere and never prints it. Absent a Shell we mint one for development,
    // which is unusable from outside this process by design.
    let token = std::env::var("ZWS_TOKEN").unwrap_or_else(|_| api::mint_token());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    // What does the work, when this build has it.
    //
    // The servers are looked for beside the sibling checkouts during development; a release
    // provisions them next to the app. If they are absent the Core still runs and says so
    // when asked, rather than failing to start.
    #[cfg(feature = "adk")]
    let state = {
        // The store comes first: the specialist remembers through it, so an Engine built
        // without it would tell the User it had remembered something and write nothing.
        let kept = keeper::Keeper::open(std::path::Path::new(&dir));
        if let Err(detail) = &kept {
            eprintln!("[core] nothing will be kept this session: {detail}");
        }

        let siblings = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let servers = studio_runner::pipeline::ServerBinaries::from_siblings(&siblings);
        let mut engine =
            studio_runner::pipeline::Engine::new(studio_router::Policy::openai_default(), servers);
        if let Ok(keeper) = &kept {
            engine = engine
                .remembering(std::sync::Arc::clone(keeper) as std::sync::Arc<_>)
                .providing(std::sync::Arc::clone(keeper) as std::sync::Arc<_>);
            // So Settings shows what is really there rather than an empty list.
            if let Err(detail) = keeper.provision(&siblings) {
                eprintln!("[core] could not record what came with Work Studio: {detail}");
            }
        }

        let api = api::Api::with_engine(&token, std::sync::Arc::new(engine));
        match kept {
            Ok(keeper) => api.with_keeper(keeper),
            Err(_) => api,
        }
    };
    #[cfg(not(feature = "adk"))]
    let state = match keeper::Keeper::open(std::path::Path::new(&dir)) {
        Ok(keeper) => api::Api::new(&token).with_keeper(keeper),
        Err(detail) => {
            eprintln!("[core] nothing will be kept this session: {detail}");
            api::Api::new(&token)
        }
    };
    println!("Listening on 127.0.0.1:{port} (token withheld from output)");

    if std::env::var("ZWS_SERVE").is_ok() {
        std::fs::write(std::path::Path::new(&dir).join("port"), port.to_string())?;
        axum::serve(listener, api::router(state)).await?;
    }

    Ok(())
}
