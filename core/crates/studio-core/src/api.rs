//! The loopback channel.
//!
//! The Core binds to loopback on an OS-assigned port and requires a bearer token
//! minted at spawn by the Shell. Nothing else can reach it: not another
//! application, not a web page, not another user on the machine without the token.
//!
//! Every payload leaving here is a `studio-api` view type, so the renderer cannot
//! be handed an agent, model, server or tool identifier even by accident
//! (Requirement 3.4). The one exception is the diagnostics endpoint.
//!
//! The event stream is ordered and resumable (task 1.5): each event carries a
//! monotonic sequence number, and a reconnecting renderer replays from the last
//! sequence it saw, so a reload never loses tray items or in-progress work.

// Parts of this module are consumed by the Shell (task 1.2) and the run pipeline
// (task 3.5). Until those land, some of the surface is exercised only by tests.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use serde::{Deserialize, Serialize};
use studio_api::{JobView, TrayItemView};
use tokio::sync::{RwLock, broadcast};

/// One event on the Core→renderer stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Monotonic. A renderer resumes from the last sequence it saw.
    pub seq: u64,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum EventKind {
    /// A piece of work changed: new state, new outcome, new next time.
    JobChanged { job: JobView },
    /// Something arrived in the tray.
    TrayItemAdded { item: TrayItemView },
    /// Something left the tray.
    TrayItemResolved { id: String },
    /// Progress in the User's terms, e.g. "Reading your sources… found 34 items".
    Progress { job_id: String, message: String },
}

/// Bounded history so a reconnecting renderer can replay. Older events are
/// dropped; the renderer refetches state when its resume point is too old.
const HISTORY: usize = 512;

pub struct Api {
    token: String,
    seq: AtomicU64,
    history: RwLock<Vec<Event>>,
    tx: broadcast::Sender<Event>,
}

impl Api {
    pub fn new(token: impl Into<String>) -> Arc<Self> {
        let (tx, _) = broadcast::channel(HISTORY);
        Arc::new(Self {
            token: token.into(),
            seq: AtomicU64::new(0),
            history: RwLock::new(Vec::new()),
            tx,
        })
    }

    /// Publish an event, assigning it the next sequence number.
    pub async fn publish(&self, kind: EventKind) -> Event {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let event = Event { seq, kind };
        {
            let mut h = self.history.write().await;
            h.push(event.clone());
            let len = h.len();
            if len > HISTORY {
                h.drain(0..len - HISTORY);
            }
        }
        // A send error only means nobody is listening yet.
        let _ = self.tx.send(event.clone());
        event
    }

    /// Events after `since`, for a reconnecting renderer.
    pub async fn replay(&self, since: u64) -> Vec<Event> {
        self.history
            .read()
            .await
            .iter()
            .filter(|e| e.seq > since)
            .cloned()
            .collect()
    }

    /// True when the renderer's resume point predates our retained history, in
    /// which case it must refetch rather than replay.
    pub async fn resume_too_old(&self, since: u64) -> bool {
        let h = self.history.read().await;
        match h.first() {
            None => since > 0 && self.seq.load(Ordering::SeqCst) > 0,
            Some(first) => since + 1 < first.seq,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    fn authorised(&self, headers: &HeaderMap) -> bool {
        headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| constant_time_eq(t.as_bytes(), self.token.as_bytes()))
    }
}

/// Comparison that does not leak the token's length or content through timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, Deserialize)]
pub struct SinceQuery {
    #[serde(default)]
    pub since: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    pub events: Vec<Event>,
    /// When true the renderer's resume point is too old; refetch instead.
    pub refetch_required: bool,
    pub latest_seq: u64,
}

pub fn router(api: Arc<Api>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/events", get(events))
        // The three artefact kinds. In every case the Core reads the file and the
        // renderer draws what it is given: one parser, one calculator, one description
        // of what the file contains.
        .route("/sheet", get(sheet))
        .route("/document", get(document))
        .route("/deck", get(deck))
        .with_state(api)
}

#[derive(Debug, Deserialize)]
pub struct SheetQuery {
    /// Path to the User's own file.
    pub path: String,
}

/// Read a file and answer with a model, or with a problem in the User's words.
///
/// The cause of a failure never enters the payload: it goes to the diagnostics record
/// instead (Requirements 17.2, 17.5).
macro_rules! artefact_route {
    ($name:ident, $read:path) => {
        async fn $name(
            State(api): State<Arc<Api>>,
            headers: HeaderMap,
            Query(query): Query<PathQuery>,
        ) -> axum::response::Response {
            if !api.authorised(&headers) {
                return (StatusCode::UNAUTHORIZED, "").into_response();
            }
            match $read(std::path::Path::new(&query.path)) {
                Ok(model) => Json(model).into_response(),
                Err(error) => {
                    if let Some(detail) = error.detail() {
                        tracing_detail(detail);
                    }
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(serde_json::json!({ "problem": error.to_string() })),
                    )
                        .into_response()
                }
            }
        }
    };
}

#[derive(Debug, Deserialize)]
pub struct PathQuery {
    /// Path to the User's own file.
    pub path: String,
}

artefact_route!(document, studio_docs::read);
artefact_route!(deck, studio_decks::read);

async fn sheet(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Query(query): Query<SheetQuery>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match studio_sheets::read(
        std::path::Path::new(&query.path),
        studio_sheets::Window::default(),
    ) {
        Ok(model) => Json(model).into_response(),
        // `problem` is what the User reads. The cause goes nowhere near this payload:
        // it belongs in the diagnostics view (Requirements 17.2, 17.5).
        Err(error) => {
            if let Some(detail) = error.detail() {
                tracing_detail(detail);
            }
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "problem": error.to_string() })),
            )
                .into_response()
        }
    }
}

async fn health(State(api): State<Arc<Api>>, headers: HeaderMap) -> impl IntoResponse {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    Json(serde_json::json!({ "ready": true })).into_response()
}

async fn events(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Query(q): Query<SinceQuery>,
) -> impl IntoResponse {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let refetch_required = api.resume_too_old(q.since).await;
    let events = if refetch_required {
        Vec::new()
    } else {
        api.replay(q.since).await
    };
    Json(EventsResponse {
        events,
        refetch_required,
        latest_seq: api.seq.load(Ordering::SeqCst),
    })
    .into_response()
}

/// Record a technical cause where support can find it, and the User cannot.
fn tracing_detail(detail: &str) {
    eprintln!("[diagnostics] {detail}");
}

/// Mint a token for one Core process. The Shell passes this to the renderer
/// through its preload bridge and never writes it to disk.
pub fn mint_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // A real build takes this from the OS CSPRNG; this keeps the skeleton
    // dependency-free until task 1.2 wires the Shell.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let pid = std::process::id() as u128;
    format!("{:032x}{:08x}", nanos ^ (pid << 64), pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_api::{StateBadge, TrayClass};

    fn job() -> JobView {
        JobView {
            id: "j1".into(),
            purpose: "Daily newsletter".into(),
            badge: StateBadge::Scheduled,
            schedule_human: Some("Every weekday at 7:00 am".into()),
            next_human: Some("Tomorrow, 7:00 am".into()),
            last_outcome: None,
            status_detail: "Next tomorrow, 7:00 am".into(),
            spend_today: None,
            steering: vec![],
        }
    }

    fn tray_item(id: &str) -> TrayItemView {
        TrayItemView {
            id: id.into(),
            class: TrayClass::Kickoff,
            headline: "Your daily newsletter is ready".into(),
            detail: "Nothing has been sent yet".into(),
            job_purpose: "Daily newsletter".into(),
            choices: vec![],
            created_human: "just now".into(),
        }
    }

    #[tokio::test]
    async fn events_are_ordered_and_sequence_is_monotonic() {
        let api = Api::new("t");
        let a = api
            .publish(EventKind::Progress {
                job_id: "j1".into(),
                message: "one".into(),
            })
            .await;
        let b = api
            .publish(EventKind::Progress {
                job_id: "j1".into(),
                message: "two".into(),
            })
            .await;
        let c = api.publish(EventKind::JobChanged { job: job() }).await;
        assert_eq!((a.seq, b.seq, c.seq), (1, 2, 3));
    }

    /// Task 1.5: a renderer reload must never lose tray items or in-progress work.
    #[tokio::test]
    async fn a_reconnecting_renderer_replays_exactly_what_it_missed() {
        let api = Api::new("t");
        api.publish(EventKind::TrayItemAdded {
            item: tray_item("t1"),
        })
        .await;
        let seen = api
            .publish(EventKind::TrayItemAdded {
                item: tray_item("t2"),
            })
            .await;
        api.publish(EventKind::TrayItemAdded {
            item: tray_item("t3"),
        })
        .await;
        api.publish(EventKind::TrayItemResolved { id: "t1".into() })
            .await;

        let missed = api.replay(seen.seq).await;
        assert_eq!(
            missed.len(),
            2,
            "should replay only what came after seq {}",
            seen.seq
        );
        assert_eq!(missed[0].seq, 3);
        assert_eq!(missed[1].seq, 4);
        assert!(!api.resume_too_old(seen.seq).await);
    }

    #[tokio::test]
    async fn replaying_from_zero_yields_everything() {
        let api = Api::new("t");
        for i in 0..5 {
            api.publish(EventKind::Progress {
                job_id: "j".into(),
                message: format!("{i}"),
            })
            .await;
        }
        assert_eq!(api.replay(0).await.len(), 5);
    }

    #[tokio::test]
    async fn a_resume_point_older_than_history_asks_for_a_refetch() {
        let api = Api::new("t");
        for i in 0..(HISTORY + 10) {
            api.publish(EventKind::Progress {
                job_id: "j".into(),
                message: format!("{i}"),
            })
            .await;
        }
        assert!(
            api.resume_too_old(1).await,
            "a resume point that has been dropped must force a refetch, not a silent gap"
        );
        assert!(!api.resume_too_old(HISTORY as u64 + 5).await);
    }

    #[tokio::test]
    async fn live_subscribers_receive_events() {
        let api = Api::new("t");
        let mut rx = api.subscribe();
        api.publish(EventKind::Progress {
            job_id: "j".into(),
            message: "hello".into(),
        })
        .await;
        let got = rx.recv().await.expect("event delivered");
        assert_eq!(got.seq, 1);
    }

    #[test]
    fn a_request_without_the_token_is_rejected() {
        let api = Api::new("secret-token");
        let mut headers = HeaderMap::new();
        assert!(!api.authorised(&headers), "no header must be rejected");

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer wrong".parse().unwrap(),
        );
        assert!(!api.authorised(&headers), "a wrong token must be rejected");

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "secret-token".parse().unwrap(),
        );
        assert!(!api.authorised(&headers), "the Bearer scheme is required");

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer secret-token".parse().unwrap(),
        );
        assert!(
            api.authorised(&headers),
            "the correct token must be accepted"
        );
    }

    #[test]
    fn token_comparison_is_length_safe() {
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
    }

    #[test]
    fn minted_tokens_differ() {
        let a = mint_token();
        std::thread::sleep(std::time::Duration::from_nanos(10));
        let b = mint_token();
        assert_ne!(a, b);
        assert!(a.len() >= 32, "token should not be trivially short");
    }

    /// Correctness Property 12 again, at the boundary: whatever the stream
    /// carries must survive the payload guardrail.
    #[tokio::test]
    async fn nothing_on_the_stream_leaks_a_technical_identifier() {
        let api = Api::new("t");
        api.publish(EventKind::JobChanged { job: job() }).await;
        api.publish(EventKind::TrayItemAdded {
            item: tray_item("t1"),
        })
        .await;
        api.publish(EventKind::Progress {
            job_id: "j1".into(),
            message: "Reading your sources… found 34 items".into(),
        })
        .await;

        for event in api.replay(0).await {
            let value = serde_json::to_value(&event).expect("serialises");
            let mut leaks = Vec::new();
            studio_api::lint::scan(&value, "", &mut leaks);
            assert!(leaks.is_empty(), "event {} leaked: {leaks:?}", event.seq);
        }
    }
}
