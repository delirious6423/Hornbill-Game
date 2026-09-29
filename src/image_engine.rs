use crate::{
    database::Database,
    image_prompt::{self, ImagePolicy, ImageRequest, ImageSettings},
    inference::{ImageBackend, process::acquire_lease},
};
use anyhow::{Context, Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io::Cursor, path::Path};

pub fn local_backend(
    root: &Path,
    runtime: &Path,
    listen_for_ctrl_c: bool,
) -> crate::inference::process::ProcessBackend {
    crate::inference::process::ProcessBackend {
        executable: runtime.join("image-venv/bin/python"),
        args: vec![
            root.join("workers/z_image/worker.py").display().to_string(),
            "--model".into(),
            runtime
                .join("models/z-image-turbo-benny")
                .display()
                .to_string(),
        ],
        timeout: std::time::Duration::from_secs(1200),
        worker_lock: root.join("data/worker.lock"),
        listen_for_ctrl_c,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneImage {
    pub attempt_id: i64,
    pub turn: u32,
    pub path: String,
    pub reason: String,
    pub request: ImageRequest,
    pub metadata: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageFile {
    path: String,
    width: u32,
    height: u32,
}

impl Database {
    pub fn latest_image(&self, save: &str) -> Result<Option<SceneImage>> {
        let row = self.connection.query_row("SELECT a.id,s.turn,a.result_json,s.reason,a.request_json,a.metadata_json FROM scene_images s JOIN image_attempts a ON a.id=s.attempt_id WHERE s.save_id=?1 AND a.status='succeeded' ORDER BY s.turn DESC LIMIT 1",[save],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,u32>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?))).optional()?;
        row.map(|(attempt_id, turn, result, reason, request, metadata)| {
            let file: ImageFile = serde_json::from_str(&result)?;
            Ok(SceneImage {
                attempt_id,
                turn,
                path: file.path,
                reason,
                request: serde_json::from_str(&request)?,
                metadata: serde_json::from_str(&metadata)?,
            })
        })
        .transpose()
    }

    pub fn image_audit(&self, save: &str) -> Result<Vec<Value>> {
        let mut q=self.connection.prepare("SELECT id,turn,request_json,status,result_json,metadata_json,error,created_at,finished_at FROM image_attempts WHERE save_id=?1 ORDER BY id")?;
        let rows=q.query_map([save],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"turn":r.get::<_,u32>(1)?,"request":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,"result":r.get::<_,Option<String>>(4)?,"metadata":r.get::<_,Option<String>>(5)?,"error":r.get::<_,Option<String>>(6)?,"created_at":r.get::<_,String>(7)?,"finished_at":r.get::<_,Option<String>>(8)?})))?;
        let mut result = Vec::new();
        for row in rows {
            let mut row = row?;
            for key in ["request", "result", "metadata"] {
                if let Some(s) = row[key].as_str() {
                    row[key] = serde_json::from_str(s)?;
                }
            }
            result.push(row);
        }
        Ok(result)
    }

    pub fn recover_images(&self, save: &str) -> Result<()> {
        self.connection.execute("UPDATE image_attempts SET status='interrupted',error='Application or image worker stopped before completion',finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE save_id=?1 AND status='running'",[save])?;
        Ok(())
    }
}

pub struct ImageEngine<'a> {
    pub database: &'a mut Database,
    pub assets: &'a Path,
    pub turn_lock: &'a Path,
}

impl ImageEngine<'_> {
    pub async fn illustrate(
        &mut self,
        save: &str,
        backend: &mut impl ImageBackend,
        policy: ImagePolicy,
        threshold: f32,
        settings: ImageSettings,
        force: bool,
    ) -> Result<Option<SceneImage>> {
        self.illustrate_expected(save, backend, policy, threshold, settings, force, None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn illustrate_expected(
        &mut self,
        save: &str,
        backend: &mut impl ImageBackend,
        policy: ImagePolicy,
        threshold: f32,
        settings: ImageSettings,
        force: bool,
        expected_turn: Option<u32>,
    ) -> Result<Option<SceneImage>> {
        let _lease = acquire_lease(self.turn_lock)?;
        self.database.recover_images(save)?;
        let state = self.database.load(save)?;
        ensure!(
            expected_turn.is_none_or(|turn| turn == state.turn),
            "the scene changed in another session; its illustration was not generated"
        );
        let scene = state
            .current_scene
            .as_ref()
            .context("advance the story before illustrating it")?;
        let previous = self.database.latest_image(save)?;
        let valid_previous =
            previous.filter(|p| image_prompt::resolve_asset(self.assets, &p.path).is_ok());
        let key = image_prompt::visual_key(&state, scene)?;
        let reason = image_prompt::render_reason(
            policy,
            threshold,
            valid_previous
                .as_ref()
                .map(|p| p.request.visual_key.as_str()),
            &key,
            scene.visual_importance,
            force,
        )?;
        if reason.is_none() {
            if let Some(mut image) = valid_previous {
                self.database.connection.execute("INSERT INTO scene_images(save_id,turn,attempt_id,reason) VALUES (?1,?2,?3,'reused') ON CONFLICT(save_id,turn) DO UPDATE SET attempt_id=excluded.attempt_id,reason=excluded.reason",params![save,state.turn,image.attempt_id])?;
                image.turn = state.turn;
                image.reason = "reused".into();
                return Ok(Some(image));
            }
            return Ok(None);
        }
        crate::state::identifier(save)?;
        let dir = self.assets.join("generations").join(save);
        std::fs::create_dir_all(&dir)?;
        self.database.connection.execute("INSERT INTO image_attempts(save_id,turn,request_json,status) VALUES (?1,?2,'{}','running')",params![save,state.turn])?;
        let attempt = self.database.connection.last_insert_rowid();
        let relative = format!("generations/{save}/turn-{}-{attempt}.png", state.turn);
        let output = self.assets.canonicalize()?.join(&relative);
        let request = match image_prompt::compile(&state, self.assets, &output, settings) {
            Ok(request) => request,
            Err(error) => {
                self.reject(attempt, &error.to_string(), &json!({}))?;
                return Err(error);
            }
        };
        self.database.connection.execute(
            "UPDATE image_attempts SET request_json=?2 WHERE id=?1",
            params![attempt, serde_json::to_string(&request)?],
        )?;
        let reply = match backend.generate_scene(&request).await {
            Ok(reply) => reply,
            Err(error) => {
                self.reject(attempt, &format!("{error:#}"), &json!({}))?;
                return Err(error);
            }
        };
        let validation = (|| -> Result<ImageFile> {
            ensure!(
                reply.protocol_version == 1 && reply.metadata.is_object(),
                "invalid image reply envelope"
            );
            ensure!(
                reply.error.is_none(),
                "image worker failed: {}",
                reply.error.as_deref().unwrap_or_default()
            );
            let file: ImageFile = serde_json::from_str(&reply.raw_text)?;
            ensure!(
                file.path == request.output_path,
                "image worker returned an unexpected path"
            );
            ensure!(
                file.width == request.settings.width && file.height == request.settings.height,
                "image dimensions do not match the request"
            );
            let bytes = std::fs::read(&output).context("image worker did not produce a file")?;
            ensure!(
                bytes.len() <= 32 * 1024 * 1024 && bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "worker output is not a valid PNG"
            );
            let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(768);
            limits.max_image_height = Some(768);
            limits.max_alloc = Some(32 * 1024 * 1024);
            reader.limits(limits);
            let decoded = reader.decode()?;
            ensure!(
                decoded.width() == file.width && decoded.height() == file.height,
                "PNG contents have different dimensions"
            );
            Ok(file)
        })();
        if let Err(error) = validation {
            self.reject(attempt, &format!("{error:#}"), &reply.metadata)?;
            return Err(error);
        }
        let stored = json!({"path":relative,"width":request.settings.width,"height":request.settings.height});
        let tx = self.database.connection.transaction()?;
        let version: u32 = tx.query_row("SELECT version FROM saves WHERE id=?1", [save], |r| {
            r.get(0)
        })?;
        ensure!(
            version == state.turn,
            "story advanced during image generation; image was not attached"
        );
        tx.execute("UPDATE image_attempts SET status='succeeded',result_json=?2,metadata_json=?3,finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![attempt,stored.to_string(),reply.metadata.to_string()])?;
        tx.execute("INSERT INTO scene_images(save_id,turn,attempt_id,reason) VALUES (?1,?2,?3,?4) ON CONFLICT(save_id,turn) DO UPDATE SET attempt_id=excluded.attempt_id,reason=excluded.reason",params![save,state.turn,attempt,reason.unwrap()])?;
        tx.commit()?;
        self.database.latest_image(save)
    }

    fn reject(&self, id: i64, error: &str, metadata: &Value) -> Result<()> {
        self.database.connection.execute("UPDATE image_attempts SET status='failed',error=?2,metadata_json=?3,finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,error,metadata.to_string()])?;
        Ok(())
    }
}
