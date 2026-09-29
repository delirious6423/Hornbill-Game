use anyhow::Result;
use hornbill::{
    database::Database,
    image_engine::ImageEngine,
    image_prompt::{self, ImagePolicy, ImageRequest, ImageSettings},
    inference::{BackendReply, DemoBackend, GenerationSettings, ImageBackend},
    state::GameState,
    story_engine::Engine,
};
use serde_json::json;
use tempfile::TempDir;

struct Picture {
    calls: usize,
    bad: bool,
}
impl ImageBackend for Picture {
    async fn generate_scene(&mut self, r: &ImageRequest) -> Result<BackendReply> {
        self.calls += 1;
        if self.bad {
            std::fs::write(&r.output_path, b"not a PNG")?;
        } else {
            image::DynamicImage::new_rgb8(r.settings.width, r.settings.height)
                .save(&r.output_path)?;
        }
        Ok(BackendReply {
            protocol_version: 1,
            raw_text:
                json!({"path":r.output_path,"width":r.settings.width,"height":r.settings.height})
                    .to_string(),
            metadata: json!({"is_ai":false}),
            error: None,
        })
    }
}

async fn story(dir: &TempDir) -> Engine {
    let db = Database::open(&dir.path().join("story.sqlite3")).unwrap();
    db.create_save("test", "Test", &GameState::initial())
        .unwrap();
    let mut engine = Engine {
        database: db,
        turn_lock: dir.path().join("turn.lock"),
        retries: 0,
    };
    engine
        .advance(
            "test",
            "I inspect the radio.",
            &mut DemoBackend,
            GenerationSettings {
                max_tokens: 1024,
                context_tokens: 6144,
                temperature: 0.6,
                seed: 42,
                memory_limit_bytes: 10 * 1024_u64.pow(3),
            },
        )
        .await
        .unwrap();
    engine
}

#[tokio::test]
async fn failed_image_keeps_story_and_last_valid_illustration() {
    let dir = TempDir::new().unwrap();
    let mut story = story(&dir).await;
    let before = story.database.load("test").unwrap();
    let lock = dir.path().join("turn.lock");
    let mut service = ImageEngine {
        database: &mut story.database,
        assets: dir.path(),
        turn_lock: &lock,
    };
    let settings = ImageSettings {
        width: 256,
        height: 256,
        ..Default::default()
    };
    let mut backend = Picture {
        calls: 0,
        bad: false,
    };
    let good = service
        .illustrate(
            "test",
            &mut backend,
            ImagePolicy::Always,
            0.65,
            settings.clone(),
            false,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(good.request.prompt.contains("Copper-red bob"));
    assert!(good.request.prompt.contains("Round brass spectacles"));
    backend.bad = true;
    assert!(
        service
            .illustrate(
                "test",
                &mut backend,
                ImagePolicy::Always,
                0.65,
                settings,
                true
            )
            .await
            .is_err()
    );
    assert_eq!(service.database.load("test").unwrap(), before);
    assert_eq!(
        service
            .database
            .latest_image("test")
            .unwrap()
            .unwrap()
            .attempt_id,
        good.attempt_id
    );
    let audit = service.database.image_audit("test").unwrap();
    assert_eq!(audit.len(), 2);
    assert_eq!(audit[1]["status"], "failed");
}

#[tokio::test]
async fn minor_scenes_reuse_the_file_without_starting_an_image_worker() {
    let dir = TempDir::new().unwrap();
    let mut story = story(&dir).await;
    let lock = dir.path().join("turn.lock");
    let mut service = ImageEngine {
        database: &mut story.database,
        assets: dir.path(),
        turn_lock: &lock,
    };
    let mut backend = Picture {
        calls: 0,
        bad: false,
    };
    let settings = ImageSettings {
        width: 256,
        height: 256,
        ..Default::default()
    };
    let first = service
        .illustrate(
            "test",
            &mut backend,
            ImagePolicy::Smart,
            1.0,
            settings.clone(),
            false,
        )
        .await
        .unwrap()
        .unwrap();
    let second = service
        .illustrate(
            "test",
            &mut backend,
            ImagePolicy::Smart,
            1.0,
            settings,
            false,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(backend.calls, 1);
    assert_eq!(first.path, second.path);
    assert_eq!(second.reason, "reused");
}

#[test]
fn scheduling_honors_manual_mode_and_canonical_identity_changes() {
    assert_eq!(
        image_prompt::render_reason(ImagePolicy::Manual, 0.7, Some("old"), "new", 1.0, false)
            .unwrap(),
        None
    );
    assert!(
        image_prompt::render_reason(ImagePolicy::Smart, 0.7, Some("old"), "new", 0.1, false)
            .unwrap()
            .is_some()
    );
    assert!(
        image_prompt::render_reason(ImagePolicy::Smart, 0.7, Some("same"), "same", 0.8, false)
            .unwrap()
            .is_some()
    );
    assert!(
        image_prompt::render_reason(ImagePolicy::Smart, f32::NAN, None, "same", 0.1, false)
            .is_err()
    );
}

#[test]
fn old_database_upgrades_without_replacing_its_save() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old.sqlite3");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../prompts/migration_001.sql"))
        .unwrap();
    c.execute(
        "INSERT INTO saves(id,title,version,state_json) VALUES ('old','Old world',0,?1)",
        [serde_json::to_string(&GameState::initial()).unwrap()],
    )
    .unwrap();
    drop(c);
    let upgraded = Database::open(&path).unwrap();
    assert_eq!(upgraded.load("old").unwrap(), GameState::initial());
    assert!(upgraded.latest_image("old").unwrap().is_none());
}

#[test]
fn image_asset_paths_cannot_escape_the_save_directory() {
    let dir = TempDir::new().unwrap();
    assert!(image_prompt::resolve_asset(dir.path(), "../secret.png").is_err());
    assert!(image_prompt::resolve_asset(dir.path(), "/tmp/secret.png").is_err());
}
