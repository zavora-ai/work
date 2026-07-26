//! The whole thing, with a real model.
//!
//! This is the test the product is for: the User asks for a change in their own words, and
//! the file on disk is different afterwards. Everything is real — the model, the capability
//! server, the gate, the file — so a pass here means the product works rather than that its
//! parts do.
//!
//! Skipped without a credential or without the servers built, and it says which, because a
//! test that passes by doing nothing is worse than one that fails. Run it with:
//!
//! ```sh
//! set -a && . ../../adk-rust/.env && set +a
//! cargo test -p studio-runner --features adk --test live -- --nocapture --ignored
//! ```
//!
//! Marked `#[ignore]` so it never runs by accident: it spends money and needs a network.

#![cfg(feature = "adk")]

use studio_router::Policy;
use studio_runner::pipeline::{Engine, Request, ServerBinaries};

fn siblings() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn have_credential() -> bool {
    std::env::var("OPENAI_API_KEY")
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("zws-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a credential and spends money"]
async fn the_user_asks_for_a_change_and_the_file_changes() {
    if !have_credential() {
        eprintln!("skipping: no credential in the environment");
        return;
    }
    let servers = ServerBinaries::from_siblings(&siblings());
    if servers.spreadsheet.is_none() {
        eprintln!("skipping: the spreadsheet server is not built");
        return;
    }

    // A spreadsheet the User already has.
    let path = temp("model.xlsx");
    {
        let mut workbook = zavora_xlsx::Workbook::new();
        let sheet = workbook.worksheet(0).unwrap();
        sheet.set_name("Summary").unwrap();
        sheet.write(0, 0, "Month").unwrap();
        sheet.write(0, 1, "Revenue").unwrap();
        sheet.write(1, 0, "July").unwrap();
        sheet.write(1, 1, 4_960_000.0).unwrap();
        sheet.write(2, 0, "August").unwrap();
        sheet.write(2, 1, 5_240_000.0).unwrap();
        workbook.save(&path).unwrap();
    }
    let before = std::fs::read(&path).unwrap();

    let engine = Engine::new(Policy::openai_default(), servers);
    let outcome = engine
        .run(&Request {
            asked: "Add a column called Growth showing each month's revenue up 12%. \
                    Use a formula, then save the file."
                .to_string(),
            artefact: path.clone(),
            steering: vec!["Keep figures as formulas so I can see the working.".to_string()],
            thread: "live-test-session".to_string(),
        })
        .await
        .expect("the work should be done");

    eprintln!("said:      {}", outcome.said);
    eprintln!("performed: {:?}", outcome.performed);
    eprintln!("refused:   {:?}", outcome.refused);

    assert!(
        !outcome.performed.is_empty(),
        "the specialist should have used its own operations, not just talked: {outcome:?}"
    );

    // The file, not the answer, is the proof.
    let after = std::fs::read(&path).unwrap();
    assert_ne!(
        before, after,
        "the file on disk must be different afterwards"
    );

    // And the Core must be able to read back what was written, without a second parser.
    let model = studio_sheets::read(&path, studio_sheets::Window::default())
        .expect("the Core should read it back");
    let sheet = model.sheets.first().expect("a sheet");
    let text: String = sheet
        .rows
        .iter()
        .flatten()
        .map(|cell| cell.display.as_str())
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        text.to_lowercase().contains("growth"),
        "the column the User asked for should be in the file: {text}"
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// Without a credential the reason must be something the User can act on, and the product
/// must not have half-done the work.
#[tokio::test(flavor = "multi_thread")]
async fn without_a_credential_nothing_is_attempted() {
    let previous = std::env::var("OPENAI_API_KEY").ok();
    // Safety: this test is the only one touching the variable, and it restores it.
    unsafe { std::env::remove_var("OPENAI_API_KEY") };

    let servers = ServerBinaries::from_siblings(&siblings());
    let path = temp("untouched.xlsx");
    {
        let mut workbook = zavora_xlsx::Workbook::new();
        workbook.worksheet(0).unwrap().write(0, 0, "Month").unwrap();
        workbook.save(&path).unwrap();
    }
    let before = std::fs::read(&path).unwrap();

    let engine = Engine::new(Policy::openai_default(), servers);
    let result = engine
        .run(&Request {
            asked: "Add a growth column".to_string(),
            artefact: path.clone(),
            steering: Vec::new(),
            thread: "no-credential".to_string(),
        })
        .await;

    match result {
        Err(error) => {
            let message = error.to_string();
            assert!(
                !message.to_lowercase().contains("key") && !message.to_lowercase().contains("api"),
                "the reason must be in the User's terms: {message}"
            );
        }
        Ok(outcome) => panic!("work should not have been attempted: {outcome:?}"),
    }

    assert_eq!(
        before,
        std::fs::read(&path).unwrap(),
        "a run that could not start must leave the file alone"
    );

    if let Some(key) = previous {
        unsafe { std::env::set_var("OPENAI_API_KEY", key) };
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
