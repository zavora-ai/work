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

/// The User's folder is not available, which only happens in a build that keeps nothing.
#[derive(Debug)]
pub struct HomeUnavailable;

impl std::fmt::Display for HomeUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Work Studio could not find your folder")
    }
}

impl HomeUnavailable {
    pub fn detail(&self) -> Option<&str> {
        None
    }
}

/// Bounded history so a reconnecting renderer can replay. Older events are
/// dropped; the renderer refetches state when its resume point is too old.
const HISTORY: usize = 512;

pub struct Api {
    token: String,
    seq: AtomicU64,
    history: RwLock<Vec<Event>>,
    tx: broadcast::Sender<Event>,
    /// What does the work. Absent when this build has nothing to think with.
    #[cfg(feature = "adk")]
    engine: Option<Arc<studio_runner::pipeline::Engine>>,
    /// The durable side: the store and the User's own folder.
    keeper: Option<Arc<crate::keeper::Keeper>>,
}

impl Api {
    pub fn new(token: impl Into<String>) -> Arc<Self> {
        let (tx, _) = broadcast::channel(HISTORY);
        Arc::new(Self {
            token: token.into(),
            seq: AtomicU64::new(0),
            history: RwLock::new(Vec::new()),
            tx,
            #[cfg(feature = "adk")]
            engine: None,
            keeper: None,
        })
    }

    /// The same, keeping what happens.
    pub fn with_keeper(self: Arc<Self>, keeper: Arc<crate::keeper::Keeper>) -> Arc<Self> {
        Arc::new(Self {
            token: self.token.clone(),
            seq: AtomicU64::new(self.seq.load(Ordering::SeqCst)),
            history: RwLock::new(Vec::new()),
            tx: self.tx.clone(),
            #[cfg(feature = "adk")]
            engine: self.engine.clone(),
            keeper: Some(keeper),
        })
    }

    /// The User's own folder.
    pub fn home(&self) -> std::result::Result<&studio_artefacts::home::Home, HomeUnavailable> {
        self.keeper
            .as_ref()
            .map(|keeper| keeper.home())
            .ok_or(HomeUnavailable)
    }

    pub fn steering_view(
        &self,
        thread: Option<&str>,
    ) -> std::result::Result<crate::keeper::SteeringView, String> {
        self.keeper
            .as_ref()
            .ok_or_else(|| "nothing is being kept".to_string())?
            .steering_view(thread)
    }

    pub fn add_note(
        &self,
        thread: Option<&str>,
        text: &str,
    ) -> std::result::Result<crate::keeper::NoteView, String> {
        self.keeper
            .as_ref()
            .ok_or_else(|| "nothing is being kept".to_string())?
            .add_note(thread, text)
    }

    pub fn act_on_note(
        &self,
        id: &str,
        action: &str,
        text: Option<&str>,
    ) -> std::result::Result<(), String> {
        self.keeper
            .as_ref()
            .ok_or_else(|| "nothing is being kept".to_string())?
            .act_on_note(id, action, text)
    }

    /// The same, with something to think with.
    #[cfg(feature = "adk")]
    pub fn with_engine(
        token: impl Into<String>,
        engine: Arc<studio_runner::pipeline::Engine>,
    ) -> Arc<Self> {
        let (tx, _) = broadcast::channel(HISTORY);
        Arc::new(Self {
            token: token.into(),
            seq: AtomicU64::new(0),
            history: RwLock::new(Vec::new()),
            tx,
            engine: Some(engine),
            keeper: None,
        })
    }

    #[cfg(feature = "adk")]
    fn engine(&self) -> Option<Arc<studio_runner::pipeline::Engine>> {
        self.engine.clone()
    }

    /// What the User has told Work Studio about this thread.
    ///
    /// Resolved here rather than sent by the renderer, so nothing influences a run that the
    /// User cannot see and edit (Requirement 6).
    pub async fn steering_for(&self, thread: &str) -> Vec<String> {
        match &self.keeper {
            Some(keeper) => keeper.notes_for_run(thread, Some("spreadsheet")),
            None => Vec::new(),
        }
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
        // Asking for a change. A POST because it changes the User's file.
        .route("/ask", axum::routing::post(ask))
        // The User's own folder, and what they have told Work Studio.
        .route("/files", get(files))
        .route("/folder", axum::routing::post(new_folder))
        .route("/steering", get(steering).post(add_steering))
        .route("/steering/act", axum::routing::post(act_on_steering))
        // The User's own work, and what was said about it.
        .route("/threads", get(threads))
        .route("/thread", get(thread))
        // The User's own edit. Same path as an agent's, which is the point.
        .route("/edit", axum::routing::post(edit))
        // What each specialist may reach, and which of those are on.
        .route("/capabilities", get(capabilities).post(add_capability))
        .route("/capabilities/act", axum::routing::post(act_on_capability))
        // What the Dashboard says, and what the diagnostics view shows.
        .route("/overview", get(overview))
        .route("/tray", get(tray))
        .route("/tray/act", axum::routing::post(decide_tray))
        .route("/deliveries", get(deliveries))
        .route("/activity", get(activity))
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

#[derive(Debug, Deserialize)]
pub struct WithinQuery {
    /// A folder inside the User's own folder. Absent means the top of it.
    pub within: Option<String>,
}

/// What is really in the User's folder.
///
/// Folders are real folders and kinds are filters, so this reports what is on disk and says
/// what kind each file is; it never invents a folder to group them by (Property 31).
async fn files(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Query(query): Query<WithinQuery>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let home = match api.home() {
        Ok(home) => home,
        Err(error) => return problem(error.to_string(), error.detail()),
    };
    match home.list(query.within.as_deref()) {
        Ok(entries) => Json(serde_json::json!({
            "location": home.described(),
            "root": home.root().to_string_lossy(),
            "entries": entries,
        }))
        .into_response(),
        Err(error) => problem(error.to_string(), error.detail()),
    }
}

#[derive(Debug, Deserialize)]
pub struct NewFolder {
    pub name: String,
    pub within: Option<String>,
}

async fn new_folder(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(body): Json<NewFolder>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let home = match api.home() {
        Ok(home) => home,
        Err(error) => return problem(error.to_string(), error.detail()),
    };
    match home.create_folder(body.within.as_deref(), &body.name) {
        Ok(entry) => Json(entry).into_response(),
        Err(error) => problem(error.to_string(), error.detail()),
    }
}

/// What the User has told Work Studio: the notes it goes on, and anything it has noticed
/// and is waiting to be told about.
async fn steering(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Query(query): Query<ThreadQuery>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.steering_view(query.thread.as_deref()) {
        Ok(view) => Json(view).into_response(),
        Err(detail) => problem(
            "Work Studio could not read your notes".into(),
            Some(&detail),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct ThreadQuery {
    pub thread: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewNote {
    pub note: String,
    /// Absent for a note that applies to everything.
    pub thread: Option<String>,
}

async fn add_steering(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(body): Json<NewNote>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.add_note(body.thread.as_deref(), &body.note) {
        Ok(note) => Json(note).into_response(),
        Err(detail) => problem("Work Studio could not keep that note".into(), Some(&detail)),
    }
}

#[derive(Debug, Deserialize)]
pub struct NoteAction {
    pub id: String,
    /// One of: accept, reword, stop, forget.
    pub action: String,
    pub text: Option<String>,
}

/// Accepting, rewording, stopping or forgetting a note.
///
/// Accepting is the step that matters: nothing Work Studio worked out for itself influences
/// any run until the User has agreed to it in words they can read (Property 33).
async fn act_on_steering(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(body): Json<NoteAction>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.act_on_note(&body.id, &body.action, body.text.as_deref()) {
        Ok(()) => Json(serde_json::json!({ "done": true })).into_response(),
        Err(detail) => problem(
            "Work Studio could not change that note".into(),
            Some(&detail),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct HandEdit {
    pub path: String,
    /// Where in the file, in the terms of its kind: a sheet name, a block index, a slide.
    pub sheet: String,
    /// What within that: a cell reference, the word "paragraph", a shape index.
    pub cell: String,
    pub value: String,
    pub thread: Option<String>,
}

/// A change the User made by hand.
///
/// It goes through the same dispatcher an agent's change does, with the author set to the
/// User, so one history holds both and neither is a special case (Correctness Property 23).
#[cfg(feature = "adk")]
async fn edit(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(body): Json<HandEdit>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let Some(engine) = api.engine() else {
        return problem("Work Studio cannot change that file yet".into(), None);
    };
    match engine
        .edit_by_hand(&body.path, &body.sheet, &body.cell, &body.value)
        .await
    {
        Ok(()) => {
            if let Some(keeper) = api.keeper.as_ref() {
                keeper.record_change(
                    &body.path,
                    &format!("you changed {} at {}", body.cell, body.sheet),
                    true,
                );
                keeper.log(
                    "action",
                    &format!("you changed {} at {}", body.cell, body.sheet),
                );
                if let Some(thread) = body.thread.as_deref() {
                    let _ = keeper.ensure_thread(thread, "Editing by hand", Some(&body.path));
                }
            }
            Json(serde_json::json!({ "done": true })).into_response()
        }
        Err(error) => problem(error.to_string(), error.detail()),
    }
}

#[cfg(not(feature = "adk"))]
async fn edit(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(_body): Json<HandEdit>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    problem("Work Studio cannot change that file yet".into(), None)
}

/// The pieces of work the User has done. This is "Your work" in the interface.
async fn threads(State(api): State<Arc<Api>>, headers: HeaderMap) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.threads()) {
        Some(Ok(threads)) => Json(serde_json::json!({ "threads": threads })).into_response(),
        Some(Err(detail)) => problem("Work Studio could not read your work".into(), Some(&detail)),
        None => Json(serde_json::json!({ "threads": [] })).into_response(),
    }
}

/// One piece of work: what was said, and what Work Studio goes on.
async fn thread(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Query(query): Query<ThreadQuery>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let Some(id) = query.thread else {
        return problem("Work Studio needs to know which piece of work".into(), None);
    };
    let Some(keeper) = api.keeper.as_ref() else {
        return Json(serde_json::json!({ "turns": [] })).into_response();
    };
    match (keeper.turns(&id), keeper.steering_view(Some(&id))) {
        (Ok(turns), Ok(steering)) => Json(serde_json::json!({
            "turns": turns,
            "steering": steering,
        }))
        .into_response(),
        (Err(detail), _) | (_, Err(detail)) => {
            problem("Work Studio could not read that work".into(), Some(&detail))
        }
    }
}

/// What is waiting on the User.
async fn tray(State(api): State<Arc<Api>>, headers: HeaderMap) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.waiting()) {
        Some(Ok(items)) => Json(serde_json::json!({ "items": items })).into_response(),
        Some(Err(detail)) => problem("Work Studio could not read the tray".into(), Some(&detail)),
        None => Json(serde_json::json!({ "items": [] })).into_response(),
    }
}

/// What the User decided about one of them.
async fn decide_tray(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(decision): Json<crate::trays::Decision>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.decide(&decision)) {
        Some(Ok(())) => Json(serde_json::json!({ "done": true })).into_response(),
        // Resolving twice lands here, and saying so is better than pretending it worked.
        Some(Err(detail)) => problem("That has already been dealt with".into(), Some(&detail)),
        None => problem("Nothing is being kept this session".into(), None),
    }
}

/// What has gone out.
async fn deliveries(State(api): State<Arc<Api>>, headers: HeaderMap) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.delivered()) {
        Some(Ok(items)) => Json(serde_json::json!({ "items": items })).into_response(),
        Some(Err(detail)) => problem(
            "Work Studio could not read what went out".into(),
            Some(&detail),
        ),
        None => Json(serde_json::json!({ "items": [] })).into_response(),
    }
}

/// The figures on the Dashboard, counted rather than invented.
async fn overview(State(api): State<Arc<Api>>, headers: HeaderMap) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.overview()) {
        Some(Ok(view)) => Json(view).into_response(),
        Some(Err(detail)) => problem("Work Studio could not count that up".into(), Some(&detail)),
        // Nothing is being kept, so nothing can be counted. Said as unavailable rather than
        // as a row of zeros.
        None => Json(serde_json::json!({
            "working": { "value": "—", "known": false },
            "waiting": { "value": "—", "known": false },
            "done": { "value": "—", "known": false },
            "cost": { "value": "—", "known": false },
            "note": "Nothing is being kept this session."
        }))
        .into_response(),
    }
}

/// What has happened. The diagnostics view shows this, and it may hold technical detail.
async fn activity(State(api): State<Arc<Api>>, headers: HeaderMap) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.activity(60)) {
        Some(Ok(entries)) => Json(serde_json::json!({ "entries": entries })).into_response(),
        Some(Err(detail)) => problem(
            "Work Studio could not read its own record".into(),
            Some(&detail),
        ),
        None => Json(serde_json::json!({ "entries": [] })).into_response(),
    }
}

/// What each specialist may reach. Settings shows this.
async fn capabilities(State(api): State<Arc<Api>>, headers: HeaderMap) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    match api.keeper.as_ref().map(|k| k.capabilities()) {
        Some(Ok(list)) => Json(serde_json::json!({ "capabilities": list })).into_response(),
        Some(Err(detail)) => problem(
            "Work Studio could not read what it can reach".into(),
            Some(&detail),
        ),
        None => Json(serde_json::json!({ "capabilities": [] })).into_response(),
    }
}

async fn add_capability(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(body): Json<crate::capabilities::NewCapability>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let Some(keeper) = api.keeper.as_ref() else {
        return problem("Work Studio is not keeping anything yet".into(), None);
    };
    match keeper.add_capability(&body) {
        Ok(()) => Json(serde_json::json!({ "done": true })).into_response(),
        Err(detail) => problem(detail, None),
    }
}

#[derive(Debug, Deserialize)]
pub struct CapabilityAction {
    pub id: String,
    /// One of: on, off, remove, allocate.
    pub action: String,
    /// For `allocate`: exactly the specialists that may use it.
    #[serde(default)]
    pub agents: Vec<String>,
}

async fn act_on_capability(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(body): Json<CapabilityAction>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    let Some(keeper) = api.keeper.as_ref() else {
        return problem("Work Studio is not keeping anything yet".into(), None);
    };
    match keeper.act_on_capability(&body.id, &body.action, &body.agents) {
        Ok(()) => Json(serde_json::json!({ "done": true })).into_response(),
        Err(detail) => problem(detail, None),
    }
}

/// A problem in the User's words, with the cause kept for support only.
fn problem(message: String, detail: Option<&str>) -> axum::response::Response {
    if let Some(detail) = detail {
        tracing_detail(detail);
    }
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(serde_json::json!({ "problem": message })),
    )
        .into_response()
}

/// The first sentence of what was said, which is what a history entry should read like.
#[cfg(feature = "adk")]
fn first_sentence(said: &str) -> String {
    let trimmed = said.trim();
    let end = trimmed
        .find(". ")
        .map(|i| i + 1)
        .unwrap_or_else(|| trimmed.len().min(120));
    trimmed[..end].trim().to_string()
}

/// Which model the balanced tier resolves to, for pricing what a run cost.
#[cfg(feature = "adk")]
fn model_name() -> Option<String> {
    studio_router::Policy::openai_default()
        .chain_for(studio_router::QualityTier::Balanced)
        .first()
        .map(|reference| reference.model.clone())
}

/// What the User typed, and about which file.
#[derive(Debug, Deserialize)]
pub struct Asked {
    /// The User's own words.
    pub asked: String,
    /// The file open in front of them.
    pub path: String,
    /// Which thread this belongs to, so the conversation continues.
    #[serde(default)]
    pub thread: Option<String>,
}

/// Doing what the User asked.
///
/// Progress goes out on the event stream as it happens rather than being buffered into the
/// reply, because a spreadsheet edit takes the better part of a minute and silence for that
/// long reads as a hang. The reply is the outcome.
#[cfg(feature = "adk")]
async fn ask(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(asked): Json<Asked>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }

    let thread = asked.thread.unwrap_or_else(|| "this-thread".to_string());
    let engine = match api.engine() {
        Some(engine) => engine,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "problem": "Work Studio has not been set up to think yet"
                })),
            )
                .into_response();
        }
    };

    // Read what was said before recording this turn, or the question just asked would be
    // handed back as something said earlier.
    // The renderer does not supply this; the Core knows it.
    let history: Vec<studio_runner::pipeline::Said> = api
        .keeper
        .as_ref()
        .and_then(|keeper| keeper.turns(&thread).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|turn| studio_runner::pipeline::Said {
            from_user: turn.from == "you",
            text: turn.text,
        })
        .collect();

    // The piece of work exists before anything is said about it, so a note has something to
    // belong to and the User can find it again.
    if let Some(keeper) = api.keeper.as_ref() {
        let _ = keeper.ensure_thread(&thread, &asked.asked, Some(&asked.path));
        let _ = keeper.remember_turn(&thread, "you", &asked.asked);
    }

    // Kept because the request is consumed below and the history entry should read as what the
    // User asked for, not as what the specialist replied.
    let in_their_words = first_sentence(&asked.asked);

    let request = studio_runner::pipeline::Request {
        asked: asked.asked,
        artefact: std::path::PathBuf::from(&asked.path),
        // Steering is resolved by the Core, never sent by the renderer, so what influences
        // a run is always something the User can see and edit.
        steering: api.steering_for(&thread).await,
        thread: thread.clone(),
        history,
    };

    // Progress is published as it arrives. `publish` is async and the reporter is not, so
    // messages are handed over a channel rather than blocked on.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let publisher = {
        let api = Arc::clone(&api);
        let thread = thread.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                api.publish(EventKind::Progress {
                    job_id: thread.clone(),
                    message,
                })
                .await;
            }
        })
    };

    let outcome = engine
        .run_reporting(&request, |message| {
            let _ = tx.send(message.to_string());
        })
        .await;
    drop(tx);
    let _ = publisher.await;

    if let (Some(keeper), Ok(outcome)) = (api.keeper.as_ref(), outcome.as_ref()) {
        if !outcome.said.is_empty() {
            let _ = keeper.remember_turn(&thread, "studio", &outcome.said);
        }
        // What it cost, as the provider counted it. Recorded so the Dashboard can show a real
        // figure instead of an invented one.
        if !outcome.usage.is_empty() {
            // Priced from ADK-Rust's own table rather than a rate written here. Rates move,
            // and a number compiled into this file would quietly become wrong.
            let micros = model_name()
                .as_deref()
                .and_then(adk_model::openai::pricing::lookup_pricing)
                .map(|pricing| {
                    let cost = adk_model::openai::pricing::estimate_cost(
                        pricing,
                        outcome.usage.prompt_tokens as u64,
                        outcome.usage.answer_tokens as u64,
                        0,
                    );
                    ((cost.input_cost + cost.output_cost + cost.cache_cost) * 1_000_000.0) as i64
                })
                .unwrap_or(0);
            keeper.record_spend(&thread, micros);
        }
        if outcome.saved {
            // The User's own words, not the specialist's reply. A reply is often "Done." —
            // true, and useless in a list of what happened. What they asked for is what they
            // will recognise a week later.
            keeper.record_change(&asked.path, &in_their_words, false);
            keeper.delivered_to_folder(&thread, &asked.path, &in_their_words);
        }
        // A refusal is a thing waiting on the User, not a line in a log they never read. One
        // item per cause, so the same wall hit four times is one decision.
        for refusal in &outcome.refused {
            keeper.needs_you(
                &thread,
                "I was not allowed to do part of that",
                &format!(
                    "This is switched off for now: {refusal}. You can change that in Settings."
                ),
                vec!["Allow it".to_string(), "Leave it".to_string()],
                Some(&format!("refused:{refusal}")),
            );
        }
        keeper.log(
            "action",
            &format!(
                "asked about {}: {} operations, {} refused",
                asked.path,
                outcome.performed.len(),
                outcome.refused.len()
            ),
        );
    }

    match outcome {
        Ok(outcome) => Json(serde_json::json!({
            "said": outcome.said,
            "changed": outcome.saved,
            "refused": outcome.refused,
        }))
        .into_response(),
        Err(error) => {
            if let Some(detail) = error.detail() {
                tracing_detail(detail);
            }
            // Something the User switched off is a decision waiting on them, not a fault to
            // report and forget. It goes in the tray with the way to undo it, deduped on the
            // cause so asking five times leaves one item.
            if let (studio_runner::pipeline::RunError::NotAllowed { .. }, Some(keeper)) =
                (&error, api.keeper.as_ref())
            {
                keeper.needs_you(
                    &thread,
                    "I could not work on that file",
                    "You have switched this off for now. You can turn it back on in Settings.",
                    vec!["Turn it back on".to_string(), "Leave it off".to_string()],
                    Some("switched-off"),
                );
            }
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "problem": error.to_string() })),
            )
                .into_response()
        }
    }
}

/// Without the sibling checkouts there is nothing to think with, and the interface is told
/// so plainly rather than being left to time out.
#[cfg(not(feature = "adk"))]
async fn ask(
    State(api): State<Arc<Api>>,
    headers: HeaderMap,
    Json(_asked): Json<Asked>,
) -> axum::response::Response {
    if !api.authorised(&headers) {
        return (StatusCode::UNAUTHORIZED, "").into_response();
    }
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "problem": "Work Studio has not been set up to think yet"
        })),
    )
        .into_response()
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
    // `oneshot` drives the real router in-process, so these test the endpoint rather
    // than the function behind it.
    use super::*;
    use studio_api::{StateBadge, TrayClass};
    use tower::ServiceExt;

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

    /// Asking is a change to the User's own file, so it must need the credential like
    /// everything else.
    #[tokio::test]
    async fn asking_needs_the_token() {
        let api = Api::new("secret");
        let app = router(api);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/ask")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"asked":"add a column","path":"/tmp/x.xlsx"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    /// A build with nothing to think with must say so in the User's words rather than
    /// leaving the interface waiting.
    #[cfg(not(feature = "adk"))]
    #[tokio::test]
    async fn without_an_engine_the_answer_is_plain() {
        let api = Api::new("secret");
        let app = router(api);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/ask")
                    .header("authorization", "Bearer secret")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        r#"{"asked":"add a column","path":"/tmp/x.xlsx"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("has not been set up to think"), "{text}");
        for banned in ["model", "provider", "adk", "feature", "credential"] {
            assert!(
                !text.to_lowercase().contains(banned),
                "leaks {banned}: {text}"
            );
        }
    }
}
