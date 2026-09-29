use crate::{inference::StoryRequest, models::*, state::GameState};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

pub struct Database {
    pub(crate) connection: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub turn: u32,
    pub text: String,
    pub tags: Vec<String>,
    pub importance: f32,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: u32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        ensure!(
            version <= 2,
            "database was created by a newer Hornbill version"
        );
        if version == 0 {
            connection.execute_batch(include_str!("../prompts/migration_001.sql"))?;
        }
        if version < 2 {
            connection.execute_batch(include_str!("../prompts/migration_002.sql"))?;
        }
        Ok(Self { connection })
    }

    pub fn create_save(&self, id: &str, title: &str, state: &GameState) -> Result<()> {
        crate::state::identifier(id)?;
        crate::state::text_field(title, "save title", 160)?;
        state.validate()?;
        ensure!(state.turn == 0, "new worlds must start at turn 0");
        self.connection
            .execute(
                "INSERT INTO saves(id,title,version,state_json) VALUES (?1,?2,0,?3)",
                params![id, title, serde_json::to_string(state)?],
            )
            .context("cannot create save (the ID may already exist)")?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<GameState> {
        let data: Option<String> = self
            .connection
            .query_row("SELECT state_json FROM saves WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let state: GameState =
            serde_json::from_str(&data.context("save not found; run 'hornbill new' first")?)?;
        state.validate()?;
        Ok(state)
    }

    pub fn latest_turn(&self, id: &str) -> Result<Option<TurnOutput>> {
        let data: Option<String> = self
            .connection
            .query_row(
                "SELECT response_json FROM turns WHERE save_id=?1 ORDER BY turn DESC LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        data.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }

    pub fn recent_scenes(&self, id: &str) -> Result<Vec<Value>> {
        let mut query = self.connection.prepare(
            "SELECT turn,action,response_json FROM turns WHERE save_id=?1 ORDER BY turn DESC LIMIT 2"
        )?;
        let rows = query.query_map([id], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (turn, action, data) = row?;
            let out: TurnOutput = serde_json::from_str(&data)?;
            let dialogue: Vec<_> = out.dialogue.iter().take(2).map(|d|json!({"speaker":d.speaker,"text":d.text.chars().take(300).collect::<String>()})).collect();
            result.push(json!({"turn":turn, "action":action, "scene":out.scene, "discoveries":out.state_changes.discovered_facts,"recent_dialogue":dialogue}));
        }
        result.reverse();
        Ok(result)
    }

    pub fn retrieve_memories(&self, id: &str, terms: &[String]) -> Result<Vec<Memory>> {
        // Bounded candidates: recent, important, and older keyword/tag matches.
        let pattern = terms
            .iter()
            .take(12)
            .map(|t| format!("%{}%", t.replace('%', "").replace('_', "\\_")))
            .collect::<Vec<_>>();
        let mut sql = String::from(
            "SELECT DISTINCT turn,text,tags_json,importance FROM memories WHERE save_id=?1 AND (id IN (SELECT id FROM memories WHERE save_id=?1 ORDER BY turn DESC,id DESC LIMIT 16) OR id IN (SELECT id FROM memories WHERE save_id=?1 ORDER BY importance DESC,turn DESC LIMIT 16)",
        );
        for i in 0..pattern.len() {
            sql.push_str(&format!(
                " OR text LIKE ?{} ESCAPE '\\' OR tags_json LIKE ?{} ESCAPE '\\'",
                i + 2,
                i + 2
            ));
        }
        sql.push_str(") ORDER BY importance DESC,turn DESC LIMIT 64");
        let mut parameters = vec![id.to_owned()];
        parameters.extend(pattern);
        let mut query = self.connection.prepare(&sql)?;
        let rows = query.query_map(rusqlite::params_from_iter(parameters), |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f32>(3)?,
            ))
        })?;
        let mut values = Vec::new();
        for row in rows {
            let (turn, text, tags, importance) = row?;
            values.push(Memory {
                turn,
                text,
                tags: serde_json::from_str(&tags)?,
                importance,
            });
        }
        values.sort_by_key(|m| {
            let searchable = format!("{} {}", m.text, m.tags.join(" ")).to_lowercase();
            let relevance = terms
                .iter()
                .filter(|t| searchable.contains(t.as_str()))
                .count() as u32;
            std::cmp::Reverse((relevance, (m.importance * 100.0) as u32, m.turn))
        });
        values.truncate(6);
        Ok(values)
    }

    pub fn start_attempt(
        &self,
        id: &str,
        before: &GameState,
        action: &str,
        attempt: u32,
        request: &StoryRequest,
    ) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO generation_attempts(save_id,base_turn,attempt,action,request_json,state_before,status) VALUES (?1,?2,?3,?4,?5,?6,'running')",
            params![id,before.turn,attempt,action,serde_json::to_string(request)?,serde_json::to_string(before)?],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn finish_attempt(
        &self,
        attempt: i64,
        raw: &str,
        metadata: &Value,
        error: &str,
    ) -> Result<()> {
        self.connection.execute(
            "UPDATE generation_attempts SET raw_response=?2,metadata_json=?3,error=?4,status='rejected',finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
            params![attempt,raw,serde_json::to_string(metadata)?,error],
        )?;
        Ok(())
    }

    pub fn recover_interrupted(&self, id: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE generation_attempts SET status='interrupted',error='Previous app or worker stopped before commit',finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE save_id=?1 AND status='running'", [id]
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_turn(
        &mut self,
        id: &str,
        before: &GameState,
        after: &GameState,
        action: &str,
        output: &TurnOutput,
        attempt: i64,
        raw: &str,
        metadata: &Value,
    ) -> Result<()> {
        after.validate()?;
        ensure!(
            after.turn == before.turn + 1,
            "invalid state version transition"
        );
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows = transaction.execute(
            "UPDATE saves SET state_json=?1,version=?2,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?3 AND version=?4",
            params![serde_json::to_string(after)?,after.turn,id,before.turn],
        )?;
        ensure!(
            rows == 1,
            "save changed during generation; generated turn was not committed"
        );
        transaction.execute(
            "INSERT INTO turns(save_id,turn,action,response_json,state_before,state_after,attempt_id) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id,after.turn,action,serde_json::to_string(output)?,serde_json::to_string(before)?,serde_json::to_string(after)?,attempt],
        )?;
        for memory in &output.memory_updates {
            transaction.execute(
                "INSERT INTO memories(save_id,turn,text,tags_json,importance) VALUES (?1,?2,?3,?4,?5)",
                params![id,after.turn,memory.text,serde_json::to_string(&memory.tags)?,memory.importance],
            )?;
        }
        let rows = transaction.execute(
            "UPDATE generation_attempts SET raw_response=?2,metadata_json=?3,status='accepted',finished_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1 AND save_id=?4 AND base_turn=?5 AND status='running'",
            params![attempt,raw,serde_json::to_string(metadata)?,id,before.turn],
        )?;
        ensure!(rows == 1, "attempt did not match the committed turn");
        transaction.commit()?;
        Ok(())
    }

    pub fn export(&self, id: &str) -> Result<Value> {
        let state = self.load(id)?;
        let mut query = self.connection.prepare(
            "SELECT id,base_turn,attempt,action,request_json,raw_response,metadata_json,state_before,status,error,started_at,finished_at FROM generation_attempts WHERE save_id=?1 ORDER BY id"
        )?;
        let rows = query.query_map([id], |r| Ok(json!({
            "id":r.get::<_,i64>(0)?, "base_turn":r.get::<_,u32>(1)?, "attempt":r.get::<_,u32>(2)?,
            "action":r.get::<_,String>(3)?, "request":r.get::<_,String>(4)?, "raw_response":r.get::<_,Option<String>>(5)?,
            "metadata":r.get::<_,Option<String>>(6)?, "state_before":r.get::<_,String>(7)?, "status":r.get::<_,String>(8)?,
            "error":r.get::<_,Option<String>>(9)?, "started_at":r.get::<_,String>(10)?, "finished_at":r.get::<_,Option<String>>(11)?
        })))?;
        let mut attempts = Vec::new();
        for row in rows {
            let mut row = row?;
            for key in ["request", "metadata", "state_before"] {
                if let Some(s) = row[key].as_str() {
                    row[key] = serde_json::from_str(s)?;
                }
            }
            attempts.push(row);
        }
        let mut query = self.connection.prepare("SELECT turn,action,response_json,state_before,state_after,attempt_id FROM turns WHERE save_id=?1 ORDER BY turn")?;
        let rows = query.query_map([id], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?;
        let mut turns = Vec::new();
        for row in rows {
            let (turn, action, response, before, after, attempt) = row?;
            turns.push(json!({"turn":turn,"action":action,"response":serde_json::from_str::<Value>(&response)?,"state_before":serde_json::from_str::<Value>(&before)?,"state_after":serde_json::from_str::<Value>(&after)?,"attempt_id":attempt}));
        }
        Ok(
            json!({"export_version":2,"save_id":id,"state":state,"turns":turns,"attempts":attempts,
            "image_attempts":self.image_audit(id)?}),
        )
    }
}
