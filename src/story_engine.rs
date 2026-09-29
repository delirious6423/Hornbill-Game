use crate::{
    database::Database,
    inference::{BackendReply, GenerationSettings, StoryBackend, process::acquire_lease},
    models::TurnOutput,
    prompt_builder,
    state::{parse_turn, text_field},
};
use anyhow::{Result, bail, ensure};
use serde_json::json;
use std::{path::PathBuf, time::Instant};

pub struct Engine {
    pub database: Database,
    pub turn_lock: PathBuf,
    pub retries: u32,
}

impl Engine {
    pub async fn advance(
        &mut self,
        save: &str,
        action: &str,
        backend: &mut impl StoryBackend,
        settings: GenerationSettings,
    ) -> Result<(TurnOutput, BackendReply)> {
        self.advance_expected(save, action, backend, settings, None)
            .await
    }

    pub async fn advance_expected(
        &mut self,
        save: &str,
        action: &str,
        backend: &mut impl StoryBackend,
        settings: GenerationSettings,
        expected_turn: Option<u32>,
    ) -> Result<(TurnOutput, BackendReply)> {
        text_field(action, "player action", 1600)?;
        ensure!(self.retries <= 2, "at most 2 repair retries are allowed");
        ensure!(
            (256..=4096).contains(&settings.max_tokens),
            "max_tokens must be 256–4096"
        );
        ensure!(
            (2048..=8192).contains(&settings.context_tokens)
                && settings.max_tokens < settings.context_tokens,
            "context must be 2048–8192 and leave space for the prompt"
        );
        ensure!(
            settings.temperature.is_finite() && (0.0..=1.5).contains(&settings.temperature),
            "temperature must be 0–1.5"
        );
        ensure!(
            (4 * 1024_u64.pow(3)..=12 * 1024_u64.pow(3)).contains(&settings.memory_limit_bytes),
            "worker memory limit must be 4–12 GiB"
        );
        let _lease = acquire_lease(&self.turn_lock)?;
        self.database.recover_interrupted(save)?;
        let before = self.database.load(save)?;
        ensure!(
            expected_turn.is_none_or(|turn| turn == before.turn),
            "this scene changed in another session; refresh before choosing an action"
        );
        let terms = prompt_builder::retrieval_terms(&before, action);
        let memories = self.database.retrieve_memories(save, &terms)?;
        let recent = self.database.recent_scenes(save)?;
        let started = Instant::now();
        let mut last_error = None;
        for attempt in 0..=self.retries {
            let mut attempt_settings = settings.clone();
            attempt_settings.seed = settings.seed.wrapping_add(attempt);
            if attempt > 0 {
                attempt_settings.temperature = 0.0;
            }
            let request = prompt_builder::build(
                &before,
                action,
                &memories,
                &recent,
                attempt_settings,
                last_error.as_deref(),
            );
            let attempt_id = self
                .database
                .start_attempt(save, &before, action, attempt, &request)?;
            let mut reply = match backend.generate(&request).await {
                Ok(reply) => reply,
                Err(error) => {
                    self.database.finish_attempt(
                        attempt_id,
                        "",
                        &json!({}),
                        &format!("{error:#}"),
                    )?;
                    return Err(error); // Runtime failures are not JSON repair opportunities.
                }
            };
            if let Some(error) = &reply.error {
                self.database.finish_attempt(
                    attempt_id,
                    &reply.raw_text,
                    &reply.metadata,
                    error,
                )?;
                bail!("inference worker: {error}");
            }
            let validated = parse_turn(&reply.raw_text)
                .and_then(|output| before.apply(&output).map(|next| (output, next)));
            match validated {
                Ok((output, after)) => {
                    reply.metadata["attempt_count"] = (attempt + 1).into();
                    reply.metadata["turn_generation_ms"] =
                        (started.elapsed().as_secs_f64() * 1000.0).into();
                    if let Err(error) = self.database.commit_turn(
                        save,
                        &before,
                        &after,
                        action,
                        &output,
                        attempt_id,
                        &reply.raw_text,
                        &reply.metadata,
                    ) {
                        self.database.finish_attempt(
                            attempt_id,
                            &reply.raw_text,
                            &reply.metadata,
                            &format!("commit failed: {error:#}"),
                        )?;
                        return Err(error);
                    }
                    return Ok((output, reply));
                }
                Err(error) => {
                    let error = format!("{error:#}");
                    self.database.finish_attempt(
                        attempt_id,
                        &reply.raw_text,
                        &reply.metadata,
                        &error,
                    )?;
                    eprintln!("Story response rejected: {error}");
                    if attempt < self.retries {
                        eprintln!("Retrying from the unchanged save…");
                    }
                    last_error = Some(error);
                }
            }
        }
        bail!(
            "no valid story after {} attempts; save unchanged. {}",
            self.retries + 1,
            last_error.unwrap_or_default()
        )
    }
}
