use anyhow::Result;
use hornbill::{
    database::Database,
    inference::{BackendReply, GenerationSettings, StoryBackend},
    models::*,
    state::{GameState, parse_turn},
    story_engine::Engine,
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn valid() -> TurnOutput {
    serde_json::from_str(include_str!("../prompts/demo_turn.json")).unwrap()
}
fn settings() -> GenerationSettings {
    GenerationSettings {
        max_tokens: 1536,
        context_tokens: 6144,
        temperature: 0.6,
        seed: 42,
        memory_limit_bytes: 10 * 1024_u64.pow(3),
    }
}

struct Queue {
    outputs: Vec<String>,
    requests: Vec<hornbill::inference::StoryRequest>,
}
impl StoryBackend for Queue {
    async fn generate(&mut self, req: &hornbill::inference::StoryRequest) -> Result<BackendReply> {
        self.requests.push(req.clone());
        Ok(BackendReply {
            protocol_version: 1,
            raw_text: self.outputs.remove(0),
            metadata: json!({"test":true}),
            error: None,
        })
    }
}
fn engine(dir: &TempDir) -> Engine {
    let database = Database::open(&dir.path().join("save.sqlite3")).unwrap();
    database
        .create_save("main", "Test", &GameState::initial())
        .unwrap();
    Engine {
        database,
        turn_lock: dir.path().join("turn.lock"),
        retries: 1,
    }
}

#[test]
fn canonical_identity_cannot_be_rewritten() {
    let before = GameState::initial();
    let mut output = valid();
    let mut mira = before.characters["mira"].clone();
    mira.appearance.hair = "Blue hair".into();
    output.state_changes.new_characters.push(mira);
    assert!(
        before
            .apply(&output)
            .unwrap_err()
            .to_string()
            .contains("canonical")
    );
    assert_eq!(before.characters["mira"].appearance.hair, "Copper-red bob");
}

#[test]
fn semantic_errors_cannot_change_the_original_state() {
    let before = GameState::initial();
    let snapshot = before.clone();
    let mut unknown = valid();
    unknown.scene.location = "nonexistent".into();
    let mut speaker = valid();
    speaker.dialogue[0].speaker = "Mira".into();
    let mut inventory = valid();
    inventory.state_changes.inventory.push(InventoryChange {
        item: "flashlight".into(),
        delta: -2,
    });
    let mut visual = valid();
    visual.scene.visual_importance = 1.1;
    let mut duplicate = valid();
    duplicate
        .state_changes
        .flags
        .push(duplicate.state_changes.flags[0].clone());
    let mut control = valid();
    control.narration = "\u{1b}[2J".into();
    for output in [unknown, speaker, inventory, visual, duplicate, control] {
        assert!(before.apply(&output).is_err());
        assert_eq!(before, snapshot);
    }
}

#[test]
fn strict_json_rejects_missing_unknown_and_trailing_data() {
    let raw = serde_json::to_string(&valid()).unwrap();
    assert!(parse_turn(&format!("```json\n{raw}\n```\n")).is_ok());
    assert!(parse_turn(&format!("Here you go: {raw}")).is_err());
    assert!(parse_turn(&format!("{raw} {{}}")).is_err());
    let mut incomplete: Value = serde_json::from_str(&raw).unwrap();
    incomplete.as_object_mut().unwrap().remove("scene");
    assert!(parse_turn(&incomplete.to_string()).is_err());
    let mut unknown: Value = serde_json::from_str(&raw).unwrap();
    unknown["state_changes"]["appearance_override"] = json!("new hair");
    assert!(parse_turn(&unknown.to_string()).is_err());
}

#[test]
fn every_fourth_turn_requires_a_memory_summary() {
    let mut before = GameState::initial();
    before.turn = 3;
    let mut output = valid();
    output.state_changes.summary = None;
    assert!(
        before
            .apply(&output)
            .unwrap_err()
            .to_string()
            .contains("summary")
    );
}

#[test]
fn model_cannot_invent_the_players_spoken_dialogue() {
    let mut output = valid();
    output.dialogue.push(Dialogue {
        speaker: "player".into(),
        text: "An unchosen line".into(),
    });
    assert!(
        GameState::initial()
            .apply(&output)
            .unwrap_err()
            .to_string()
            .contains("NPC speech only")
    );
}

#[tokio::test]
async fn malformed_reply_is_audited_then_repaired_from_unchanged_state() {
    let dir = TempDir::new().unwrap();
    let mut engine = engine(&dir);
    let mut queue = Queue {
        outputs: vec!["{broken".into(), serde_json::to_string(&valid()).unwrap()],
        requests: vec![],
    };
    engine
        .advance("main", "Listen to the radio", &mut queue, settings())
        .await
        .unwrap();
    assert_eq!(engine.database.load("main").unwrap().turn, 1);
    assert_eq!(queue.requests[0].settings.seed, 42);
    assert_eq!(queue.requests[1].settings.seed, 43);
    assert_eq!(queue.requests[1].settings.temperature, 0.0);
    assert!(
        queue.requests[1].messages[1]
            .content
            .contains("SAME unchanged state")
    );
    let export = engine.database.export("main").unwrap();
    assert_eq!(export["attempts"][0]["status"], "rejected");
    assert_eq!(export["attempts"][1]["status"], "accepted");
    assert_eq!(
        export["attempts"][0]["state_before"],
        export["attempts"][1]["state_before"]
    );
    drop(engine);
    let reopened = Database::open(&dir.path().join("save.sqlite3")).unwrap();
    assert!(reopened.load("main").unwrap().flags["heard_three_knocks"]);
    assert_eq!(
        reopened.latest_turn("main").unwrap().unwrap().choices.len(),
        3
    );
}

#[tokio::test]
async fn exhausted_retries_do_not_advance_save_or_append_scenes() {
    let dir = TempDir::new().unwrap();
    let mut engine = engine(&dir);
    let before = engine.database.load("main").unwrap();
    let mut queue = Queue {
        outputs: vec!["{}".into(), "{}".into()],
        requests: vec![],
    };
    assert!(
        engine
            .advance("main", "Open the door", &mut queue, settings())
            .await
            .is_err()
    );
    assert_eq!(engine.database.load("main").unwrap(), before);
    let export = engine.database.export("main").unwrap();
    assert_eq!(export["turns"].as_array().unwrap().len(), 0);
    assert_eq!(export["attempts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn repair_identifies_a_nested_schema_error_without_relaxing_the_contract() {
    let dir = TempDir::new().unwrap();
    let mut engine = engine(&dir);
    let mut malformed = serde_json::to_value(valid()).unwrap();
    malformed["choices"][0] = json!({"text":"Inspect the radio", "next_action":"inspect_radio"});
    let mut queue = Queue {
        outputs: vec![
            malformed.to_string(),
            serde_json::to_string(&valid()).unwrap(),
        ],
        requests: vec![],
    };
    engine
        .advance("main", "Inspect the radio", &mut queue, settings())
        .await
        .unwrap();
    let repair = &queue.requests[1].messages[1].content;
    assert!(repair.contains("choices[0]"));
    assert!(repair.contains("expected a string"));
    let audit = engine.database.export("main").unwrap();
    assert_eq!(audit["attempts"][0]["status"], "rejected");
    assert_eq!(
        audit["attempts"][0]["state_before"],
        audit["attempts"][1]["state_before"]
    );
    assert_eq!(audit["turns"].as_array().unwrap().len(), 1);
}

#[test]
fn stale_commit_rolls_back_all_writes() {
    let dir = TempDir::new().unwrap();
    let mut e = engine(&dir);
    let before = GameState::initial();
    let output = valid();
    let after = before.apply(&output).unwrap();
    let req = hornbill::prompt_builder::build(&before, "Act", &[], &[], settings(), None);
    let first = e
        .database
        .start_attempt("main", &before, "Act", 0, &req)
        .unwrap();
    e.database
        .commit_turn(
            "main",
            &before,
            &after,
            "Act",
            &output,
            first,
            "raw",
            &json!({}),
        )
        .unwrap();
    let stale = e
        .database
        .start_attempt("main", &before, "Act", 0, &req)
        .unwrap();
    assert!(
        e.database
            .commit_turn(
                "main",
                &before,
                &after,
                "Act",
                &output,
                stale,
                "raw",
                &json!({})
            )
            .is_err()
    );
    let export = e.database.export("main").unwrap();
    assert_eq!(export["turns"].as_array().unwrap().len(), 1);
    assert_eq!(export["state"]["turn"], 1);
}

#[test]
fn bad_attempt_rolls_back_a_state_update_inside_the_transaction() {
    let dir = TempDir::new().unwrap();
    let mut e = engine(&dir);
    let before = GameState::initial();
    let output = valid();
    let after = before.apply(&output).unwrap();
    assert!(
        e.database
            .commit_turn(
                "main",
                &before,
                &after,
                "Act",
                &output,
                999,
                "raw",
                &json!({})
            )
            .is_err()
    );
    assert_eq!(e.database.load("main").unwrap(), before);
    assert!(e.database.latest_turn("main").unwrap().is_none());
}

#[test]
fn active_context_does_not_grow_with_full_world_or_transcript() {
    let mut state = GameState::initial();
    for i in 0..2000 {
        state
            .discovered_facts
            .push(format!("Old distant event {i}"));
        state.locations.insert(
            format!("place_{i}"),
            Location {
                id: format!("place_{i}"),
                name: format!("Place {i}"),
                description: "Far away".into(),
            },
        );
    }
    let req = hornbill::prompt_builder::build(
        &state,
        "Examine the observatory",
        &[],
        &[],
        settings(),
        None,
    );
    let view: Value = serde_json::from_str(&req.messages[1].content).unwrap();
    assert_eq!(view["known_facts"].as_array().unwrap().len(), 10);
    assert_eq!(view["locations"].as_array().unwrap().len(), 5);
    assert!(req.messages.iter().map(|m| m.content.len()).sum::<usize>() < 24000);
    assert!(!req.messages[1].content.contains("Old distant event 1999"));
}

#[test]
fn save_creation_never_overwrites_an_existing_game() {
    let dir = TempDir::new().unwrap();
    let e = engine(&dir);
    assert!(
        e.database
            .create_save("main", "Overwrite", &GameState::initial())
            .is_err()
    );
    assert_eq!(e.database.load("main").unwrap(), GameState::initial());
}

#[tokio::test]
async fn a_stale_displayed_choice_does_not_start_a_generation() {
    let dir = TempDir::new().unwrap();
    let mut engine = engine(&dir);
    let error = engine
        .advance_expected(
            "main",
            "I open the door.",
            &mut hornbill::inference::DemoBackend,
            settings(),
            Some(9),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("changed in another session"));
    assert_eq!(engine.database.load("main").unwrap().turn, 0);
    assert!(
        engine.database.export("main").unwrap()["attempts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn world_notes_are_app_owned_and_retrieved_in_a_bounded_subset() {
    let mut state = GameState::initial();
    for i in 0..100 {
        state.lorebook.push(hornbill::state::LoreEntry {
            id: format!("note_{i}"),
            text: format!("A distant legend numbered {i}."),
            tags: vec![],
        });
    }
    state.lorebook.push(hornbill::state::LoreEntry {
        id: "radio".into(),
        text: "Dr. Aris built the portable radio with Mira.".into(),
        tags: vec!["mira".into(), "radio".into()],
    });
    let request = hornbill::prompt_builder::build(
        &state,
        "I inspect the radio with Mira.",
        &[],
        &[],
        settings(),
        None,
    );
    let view: Value = serde_json::from_str(&request.messages[1].content).unwrap();
    assert_eq!(view["world_notes"].as_array().unwrap().len(), 4);
    assert!(view["world_notes"].to_string().contains("Dr. Aris"));
    let after = state.apply(&valid()).unwrap();
    assert_eq!(after.lorebook, state.lorebook);
}

#[test]
fn retrieval_uses_structured_events_without_copying_old_narrative_style() {
    let dir = TempDir::new().unwrap();
    let mut e = engine(&dir);
    let before = GameState::initial();
    let mut output = valid();
    output.narration = "STYLE_MARKER_SHOULD_STAY_IN_THE_AUDIT".into();
    let after = before.apply(&output).unwrap();
    let req = hornbill::prompt_builder::build(&before, "Act", &[], &[], settings(), None);
    let attempt = e
        .database
        .start_attempt("main", &before, "Act", 0, &req)
        .unwrap();
    e.database
        .commit_turn(
            "main",
            &before,
            &after,
            "Act",
            &output,
            attempt,
            "raw",
            &json!({}),
        )
        .unwrap();
    let recent = e.database.recent_scenes("main").unwrap();
    let serialized = serde_json::to_string(&recent).unwrap();
    assert!(!serialized.contains("STYLE_MARKER"));
    assert!(serialized.contains("three knocks"));
    assert_eq!(
        e.database.latest_turn("main").unwrap().unwrap().narration,
        output.narration
    );
}
