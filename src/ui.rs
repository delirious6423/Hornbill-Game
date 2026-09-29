use crate::{
    database::Database,
    image_engine::{ImageEngine, local_backend},
    image_prompt::{ImagePolicy, ImageSettings, resolve_asset},
    inference::{
        GenerationSettings,
        process::{ProcessBackend, acquire_lease},
    },
    state::{GameState, LoreEntry},
    story_engine::Engine,
};
use anyhow::{Context, Result, ensure};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{Cursor, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub model: String,
    pub image_policy: ImagePolicy,
    pub visual_threshold: f32,
    pub image: ImageSettings,
    #[serde(default = "legacy_image_backend_version")]
    pub image_backend_version: u32,
}
fn legacy_image_backend_version() -> u32 {
    1
}

#[cfg(test)]
mod preference_tests {
    use super::*;

    #[test]
    fn old_image_settings_migrate_without_changing_story_or_seed() {
        let old = json!({"model":"e4b","image_policy":"manual","visual_threshold":0.8,
            "image":{"width":384,"height":576,"steps":40,"seed":123}});
        let settings: Preferences = serde_json::from_value(old).unwrap();
        let settings = settings.migrate();
        settings.validate().unwrap();
        assert_eq!(settings.image_backend_version, 2);
        assert_eq!(settings.image.steps, 9);
        assert_eq!(settings.model, "e4b");
        assert_eq!(settings.image.seed, 123);
        assert_eq!(settings.image.width, 384);
        assert_eq!(settings.image_policy, ImagePolicy::Manual);
    }

    #[test]
    fn current_image_settings_keep_an_explicit_step_choice() {
        let current = json!({"image_backend_version":2,"image":{"steps":6}});
        let settings: Preferences = serde_json::from_value(current).unwrap();
        let settings = settings.migrate();
        settings.validate().unwrap();
        assert_eq!(settings.image.steps, 6);
    }
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            model: "12b".into(),
            image_policy: ImagePolicy::Smart,
            visual_threshold: 0.65,
            image: ImageSettings::default(),
            image_backend_version: 2,
        }
    }
}
impl Preferences {
    fn migrate(mut self) -> Self {
        if self.image_backend_version == 1 {
            self.image.steps = ImageSettings::default().steps;
            self.image_backend_version = 2;
        }
        self
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.image_backend_version == 2,
            "unknown image backend version"
        );
        ensure!(
            ["12b", "e4b", "gguf"].contains(&self.model.as_str()),
            "unknown story model"
        );
        ensure!(
            self.visual_threshold.is_finite() && (0.0..=1.0).contains(&self.visual_threshold),
            "invalid visual threshold"
        );
        self.image.validate()
    }
}

#[derive(Clone, Default, Serialize)]
struct Job {
    id: u64,
    save: String,
    running: bool,
    phase: String,
    error: Option<String>,
    started_ms: u64,
    #[serde(skip)]
    abort: Option<tokio::task::AbortHandle>,
}
#[derive(Clone)]
struct App {
    root: Arc<PathBuf>,
    data: Arc<PathBuf>,
    runtime: Arc<PathBuf>,
    token: Arc<String>,
    origin: Arc<String>,
    job: Arc<Mutex<Job>>,
}
impl App {
    fn db(&self) -> Result<Database> {
        Database::open(&self.data.join("hornbill.sqlite3"))
    }
    fn idle(&self) -> Result<()> {
        ensure!(
            !self.job.lock().unwrap().running,
            "a scene is already being created; wait or cancel it"
        );
        Ok(())
    }
    fn phase(&self, id: u64, phase: &str) {
        let mut job = self.job.lock().unwrap();
        if job.id == id {
            job.phase = phase.into();
        }
    }
}
struct ApiError(anyhow::Error);
impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(value: E) -> Self {
        Self(value.into())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{:#}",self.0)})),
        )
            .into_response()
    }
}
type Api<T> = std::result::Result<T, ApiError>;

async fn authenticate(State(app): State<App>, request: Request, next: Next) -> Response {
    if request.uri().path().starts_with("/api/") {
        let provided = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", app.token);
        if provided != Some(&expected) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error":"Open Hornbill using its launch link."})),
            )
                .into_response();
        }
        if let Some(origin) = request.headers().get(header::ORIGIN)
            && origin.to_str().ok() != Some(app.origin.as_str())
        {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY,"default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'".parse().unwrap());
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response
}

fn preferences(db: &Database, save: &str) -> Result<Preferences> {
    let value: Option<String> = db
        .connection
        .query_row(
            "SELECT settings_json FROM save_preferences WHERE save_id=?1",
            [save],
            |r| r.get(0),
        )
        .optional()?;
    let settings: Preferences = value
        .map(|s| serde_json::from_str(&s).map_err(anyhow::Error::from))
        .unwrap_or_else(|| Ok(Preferences::default()))?;
    Ok(settings.migrate())
}
fn write_preferences(db: &Database, save: &str, value: &Preferences) -> Result<()> {
    value.validate()?;
    db.load(save)?;
    db.connection.execute("INSERT INTO save_preferences(save_id,settings_json) VALUES (?1,?2) ON CONFLICT(save_id) DO UPDATE SET settings_json=excluded.settings_json",params![save,serde_json::to_string(value)?])?;
    Ok(())
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

async fn saves(State(app): State<App>) -> Api<Json<Value>> {
    let db = app.db()?;
    let mut q = db.connection.prepare(
        "SELECT id,title,version,updated_at FROM saves ORDER BY updated_at DESC LIMIT 100",
    )?;
    let rows=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"turn":r.get::<_,u32>(2)?,"updated_at":r.get::<_,String>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(Json(
        json!({"saves":rows,"models":{"12b":crate::model_profiles::installed(&app.runtime,"12b"),"e4b":crate::model_profiles::installed(&app.runtime,"e4b"),"gguf":app.runtime.join("models/gguf-12b/gemma-4-12b-it-Q4_K_M.gguf").exists()},"images_ready":app.runtime.join("models/z-image-turbo-benny/hornbill-model.json").exists()}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewSave {
    title: String,
    premise: Option<String>,
}
async fn new_save(State(app): State<App>, Json(body): Json<NewSave>) -> Api<Json<Value>> {
    app.idle()?;
    let _lease = acquire_lease(&app.data.join("turn.lock"))?;
    let id = format!("story_{}", now_ms());
    let mut state = GameState::initial();
    if let Some(p) = body.premise.filter(|p| !p.trim().is_empty()) {
        state.premise = p;
    }
    app.db()?.create_save(&id, &body.title, &state)?;
    Ok(Json(json!({"id":id})))
}
async fn state(State(app): State<App>, Path(save): Path<String>) -> Api<Json<Value>> {
    let db = app.db()?;
    Ok(Json(db.read_snapshot(|db| {
        Ok(json!({"state":db.load(&save)?,"turn":db.latest_turn(&save)?,"image":db.latest_image(&save)?,"preferences":preferences(db,&save)?}))
    })?))
}
async fn set_preferences(
    State(app): State<App>,
    Path(save): Path<String>,
    Json(p): Json<Preferences>,
) -> Api<Json<Value>> {
    app.idle()?;
    let _lease = acquire_lease(&app.data.join("turn.lock"))?;
    write_preferences(&app.db()?, &save, &p.migrate())?;
    Ok(Json(json!({"saved":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Notes {
    director_note: Option<String>,
    lorebook: Vec<LoreEntry>,
}
async fn notes(
    State(app): State<App>,
    Path(save): Path<String>,
    Json(body): Json<Notes>,
) -> Api<Json<Value>> {
    app.idle()?;
    let _lease = acquire_lease(&app.data.join("turn.lock"))?;
    let db = app.db()?;
    let mut state = db.load(&save)?;
    state.director_note = body.director_note.filter(|s| !s.trim().is_empty());
    state.lorebook = body.lorebook;
    state.validate()?;
    db.connection.execute("UPDATE saves SET state_json=?2,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![save,serde_json::to_string(&state)?])?;
    Ok(Json(json!({"saved":true})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    action: Option<String>,
    expected_turn: u32,
}
async fn turn(
    State(app): State<App>,
    Path(save): Path<String>,
    Json(body): Json<Action>,
) -> Api<Json<Value>> {
    body.action
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .context("enter an action")?;
    start_job(app, save, body, false)
}
async fn illustrate(
    State(app): State<App>,
    Path(save): Path<String>,
    Json(body): Json<Action>,
) -> Api<Json<Value>> {
    start_job(app, save, body, true)
}
fn start_job(app: App, save: String, body: Action, only_image: bool) -> Api<Json<Value>> {
    let mut job = app.job.lock().unwrap();
    (!job.running)
        .then_some(())
        .context("a scene is already being created")?;
    let db = app.db()?;
    let current = db.load(&save)?;
    (current.turn == body.expected_turn)
        .then_some(())
        .context("this scene changed; refresh before continuing")?;
    let prefs = preferences(&db, &save)?;
    prefs.validate()?;
    let id = now_ms();
    *job = Job {
        id,
        save: save.clone(),
        running: true,
        phase: if only_image {
            "Creating illustration…"
        } else {
            "Writing the next scene…"
        }
        .into(),
        error: None,
        started_ms: now_ms(),
        abort: None,
    };
    let worker_app = app.clone();
    let worker_save = save.clone();
    let task =
        tokio::spawn(
            async move { run_job(worker_app, worker_save, body, prefs, only_image, id).await },
        );
    job.abort = Some(task.abort_handle());
    drop(job);
    let watcher = app.clone();
    tokio::spawn(async move {
        let result = task.await;
        let (phase, error) = match result {
            Ok(Ok(())) => ("Scene saved", None),
            Ok(Err(e)) => ("Needs attention", Some(format!("{e:#}"))),
            Err(e) if e.is_cancelled() => (
                "Cancelled",
                Some("Cancelled. Any completed story turn is still saved.".into()),
            ),
            Err(e) => ("Needs attention", Some(format!("Scene task stopped: {e}"))),
        };
        if let Ok(_lease) = acquire_lease(&watcher.data.join("turn.lock"))
            && let Ok(db) = watcher.db()
        {
            let _ = db.recover_interrupted(&save);
            let _ = db.recover_images(&save);
        }
        let mut job = watcher.job.lock().unwrap();
        if job.id == id {
            job.running = false;
            job.phase = phase.into();
            job.error = error;
            job.abort = None;
        }
    });
    Ok(Json(json!({"started":true,"id":id})))
}
async fn run_job(
    app: App,
    save: String,
    body: Action,
    prefs: Preferences,
    only_image: bool,
    id: u64,
) -> Result<()> {
    let mut engine = Engine {
        database: app.db()?,
        turn_lock: app.data.join("turn.lock"),
        retries: 1,
    };
    let mut expected = body.expected_turn;
    if !only_image {
        let mut args = vec![
            app.root
                .join(if prefs.model == "gguf" {
                    "workers/gguf/worker.py"
                } else {
                    "workers/gemma/worker.py"
                })
                .display()
                .to_string(),
            "--model".into(),
        ];
        let model = if prefs.model == "gguf" {
            app.runtime
                .join("models/gguf-12b/gemma-4-12b-it-Q4_K_M.gguf")
        } else {
            ensure!(
                crate::model_profiles::installed(&app.runtime, &prefs.model),
                "selected story model is not installed; run scripts/model.sh download"
            );
            crate::model_profiles::path(&app.runtime, &prefs.model)?
        };
        ensure!(model.exists(), "selected story model is not installed");
        args.push(model.display().to_string());
        if prefs.model == "gguf" {
            args.extend([
                "--llama-binary".into(),
                std::fs::read_to_string(app.runtime.join("llama-binary"))?
                    .trim()
                    .into(),
            ]);
        }
        let mut backend = ProcessBackend {
            executable: app.runtime.join("venv/bin/python"),
            args,
            timeout: Duration::from_secs(360),
            worker_lock: app.root.join("data/worker.lock"),
            listen_for_ctrl_c: false,
        };
        engine
            .advance_expected(
                &save,
                body.action.as_deref().context("enter an action")?,
                &mut backend,
                GenerationSettings {
                    max_tokens: 1536,
                    context_tokens: 6144,
                    temperature: 0.6,
                    seed: 42,
                    memory_limit_bytes: 10 * 1024_u64.pow(3),
                },
                Some(expected),
            )
            .await?;
        expected += 1;
    }
    app.phase(id, "Creating illustration…");
    // The story is committed and its worker reaped before an image worker can start.
    // A competing CLI turn must never cause an illustration of the wrong scene.
    let mut images = ImageEngine {
        database: &mut engine.database,
        assets: &app.data,
        turn_lock: &engine.turn_lock,
    };
    images
        .illustrate_expected(
            &save,
            &mut local_backend(&app.root, &app.runtime, false),
            prefs.image_policy,
            prefs.visual_threshold,
            prefs.image,
            only_image,
            Some(expected),
        )
        .await
        .context("Story saved. Illustration could not finish")?;
    Ok(())
}
async fn job(State(app): State<App>) -> Json<Job> {
    Json(app.job.lock().unwrap().clone())
}
async fn cancel(State(app): State<App>) -> Json<Value> {
    let mut j = app.job.lock().unwrap();
    if let Some(a) = &j.abort {
        a.abort();
        j.phase = "Cancelling…".into();
    }
    Json(json!({"requested":true}))
}

async fn image_file(State(app): State<App>, Path(id): Path<i64>) -> Api<Response> {
    let db = app.db()?;
    let value: String = db.connection.query_row(
        "SELECT result_json FROM image_attempts WHERE id=?1 AND status='succeeded'",
        [id],
        |r| r.get(0),
    )?;
    let value: Value = serde_json::from_str(&value)?;
    let path = resolve_asset(
        &app.data,
        value["path"].as_str().context("image path missing")?,
    )?;
    Ok((
        [(header::CONTENT_TYPE, "image/png")],
        tokio::fs::read(path).await?,
    )
        .into_response())
}
async fn reference(
    State(app): State<App>,
    Path((save, entity)): Path<(String, String)>,
    bytes: Bytes,
) -> Api<Json<Value>> {
    app.idle()?;
    let _lease = acquire_lease(&app.data.join("turn.lock"))?;
    let db = app.db()?;
    let mut state = db.load(&save)?;
    (state.characters.contains_key(&entity) || state.locations.contains_key(&entity))
        .then_some(())
        .context("unknown character or place")?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?.thumbnail(1536, 1536);
    let relative = format!("references/{save}/{entity}_{}.png", now_ms());
    let path = app.data.join(&relative);
    std::fs::create_dir_all(path.parent().context("missing reference directory")?)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    image.write_to(&mut file, image::ImageFormat::Png)?;
    file.sync_all()?;
    state.references.insert(entity, vec![relative]);
    state.validate()?;
    db.connection.execute(
        "UPDATE saves SET state_json=?2 WHERE id=?1",
        params![save, serde_json::to_string(&state)?],
    )?;
    Ok(Json(json!({"saved":true})))
}
async fn export(State(app): State<App>, Path(save): Path<String>) -> Api<Json<Value>> {
    Ok(Json(app.db()?.export(&save)?))
}

pub async fn serve(root: PathBuf, data: PathBuf, port: u16, no_open: bool) -> Result<()> {
    std::fs::create_dir_all(&data)?;
    let data = data.canonicalize()?;
    let _ui_lease = acquire_lease(&data.join("ui.lock")).context("Hornbill may already be open")?;
    let runtime = std::env::var_os("HORNBILL_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".local"));
    let mut random = [0u8; 24];
    getrandom::fill(&mut random).map_err(|e| anyhow::anyhow!("session key: {e}"))?;
    let token = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let origin = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let url = format!("{origin}/#key={token}");
    let app = App {
        root: Arc::new(root),
        data: Arc::new(data),
        runtime: Arc::new(runtime),
        token: Arc::new(token),
        origin: Arc::new(origin.clone()),
        job: Arc::new(Mutex::new(Job::default())),
    };
    let routes = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../ui/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../ui/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../ui/style.css"),
                )
            }),
        )
        .route("/api/saves", get(saves).post(new_save))
        .route("/api/state/{save}", get(state))
        .route("/api/turn/{save}", post(turn))
        .route("/api/illustrate/{save}", post(illustrate))
        .route("/api/preferences/{save}", put(set_preferences))
        .route("/api/notes/{save}", put(notes))
        .route("/api/reference/{save}/{entity}", post(reference))
        .route("/api/image/{id}", get(image_file))
        .route("/api/export/{save}", get(export))
        .route("/api/job", get(job))
        .route("/api/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(app.clone(), authenticate))
        .with_state(app.clone());
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut session = options.open(app.data.join("ui-session.json"))?;
    serde_json::to_writer(
        &mut session,
        &json!({"url":url,"origin":origin,"token":app.token.as_str(),"pid":std::process::id()}),
    )?;
    session.flush()?;
    println!("Hornbill is ready: {url}");
    if !no_open {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&url).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        }
    }
    axum::serve(listener, routes)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            if let Some(abort) = &app.job.lock().unwrap().abort {
                abort.abort();
            }
        })
        .await?;
    Ok(())
}
