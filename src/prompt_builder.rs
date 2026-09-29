use crate::{
    database::Memory,
    inference::{GenerationSettings, Message, StoryRequest},
    models::*,
    state::GameState,
};
use serde::Serialize;
use serde_json::{Value, json};

pub fn retrieval_terms(state: &GameState, action: &str) -> Vec<String> {
    let mut terms = vec![state.world.location.clone()];
    if let Some(scene) = &state.current_scene {
        terms.extend(scene.characters.iter().map(|c| c.id.clone()));
    }
    terms.extend(
        action
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| s.len() >= 4)
            .take(10)
            .map(str::to_lowercase),
    );
    terms.sort();
    terms.dedup();
    terms
}

fn select<'a, T: Serialize>(
    values: impl Iterator<Item = &'a T>,
    terms: &[String],
    limit: usize,
) -> Vec<&'a T> {
    let mut scored: Vec<_> = values
        .map(|v| {
            let searchable = serde_json::to_string(v).unwrap_or_default().to_lowercase();
            let score = terms
                .iter()
                .filter(|t| searchable.contains(t.as_str()))
                .count();
            (score, v)
        })
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().take(limit).map(|(_, v)| v).collect()
}

fn select_map<'a, T>(
    values: &'a std::collections::BTreeMap<String, T>,
    terms: &[String],
    limit: usize,
) -> std::collections::BTreeMap<&'a String, &'a T> {
    let mut entries: Vec<_> = values.iter().collect();
    entries.sort_by_key(|(key, _)| {
        std::cmp::Reverse(terms.iter().filter(|t| key.contains(t.as_str())).count())
    });
    entries.into_iter().take(limit).collect()
}

pub fn build(
    state: &GameState,
    action: &str,
    memories: &[Memory],
    recent: &[Value],
    settings: GenerationSettings,
    repair: Option<&str>,
) -> StoryRequest {
    let terms = retrieval_terms(state, action);
    let mut character_terms = terms.clone();
    character_terms.push("player".into());
    // All durable state remains in SQLite. Only this bounded projection is sent.
    let mut view = json!({
        "turn":state.turn, "premise":state.premise, "world":state.world,
        "characters":select(state.characters.values(), &character_terms, 5),
        "locations":select(state.locations.values(), &terms, 5),
        "inventory":select_map(&state.inventory, &terms, 24),
        "flags":select_map(&state.flags, &terms, 24),
        "relationships":select(state.relationships.iter(), &terms, 8),
        "unresolved_plot_threads":select(state.plot_threads.values().filter(|t| t.status == ThreadStatus::Open), &terms, 6),
        "active_objectives":select(state.objectives.values().filter(|o| o.status == ObjectiveStatus::Active), &terms, 6),
        "known_facts":select(state.discovered_facts.iter(), &terms, 10),
        "long_term_summary":state.summary, "relevant_memories":memories,
        "recent_scenes":recent,
        "summary_refresh_required":(state.turn + 1).is_multiple_of(4),
        "player_action":action
    });
    if !state.lorebook.is_empty() {
        view["world_notes"] = json!(select(state.lorebook.iter(), &terms, 4));
    }
    if let Some(note) = &state.director_note {
        view["story_direction"] = json!(note);
    }
    let schema = output_schema();
    let mut system = include_str!("../prompts/story_system.txt").to_string();
    system.push_str("\nOUTPUT_JSON_SCHEMA:\n");
    system.push_str(&serde_json::to_string(&schema).expect("schema"));
    let mut content = serde_json::to_string(&view).expect("active context");
    if let Some(error) = repair {
        content.push_str("\nYour previous attempt was rejected by the application. Generate a fresh complete turn from the SAME unchanged state and action. Fix this validation error: ");
        content.extend(error.chars().take(700));
        content.push_str("\nBe concise. Return only the complete JSON object.");
    }
    StoryRequest {
        protocol_version: 1,
        messages: vec![
            Message {
                role: "system".into(),
                content: system,
            },
            Message {
                role: "user".into(),
                content,
            },
        ],
        schema,
        settings,
    }
}
