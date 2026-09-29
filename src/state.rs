use crate::models::*;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub location: String,
    pub time: String,
    pub weather: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GameState {
    pub schema_version: u32,
    pub turn: u32,
    pub premise: String,
    pub art_style: String,
    pub world: World,
    pub characters: BTreeMap<String, Character>,
    pub locations: BTreeMap<String, Location>,
    pub inventory: BTreeMap<String, u32>,
    pub flags: BTreeMap<String, bool>,
    pub relationships: Vec<Relationship>,
    pub plot_threads: BTreeMap<String, PlotThread>,
    pub objectives: BTreeMap<String, Objective>,
    pub discovered_facts: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub lorebook: Vec<LoreEntry>,
    #[serde(default)]
    pub director_note: Option<String>,
    /// App/user-owned reference paths, keyed by canonical character/location ID.
    /// There is deliberately no model output operation that changes these paths.
    pub references: BTreeMap<String, Vec<String>>,
    pub current_scene: Option<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoreEntry {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
}

pub fn text_field(value: &str, name: &str, max: usize) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{name} must not be empty");
    ensure!(
        value.chars().count() <= max,
        "{name} exceeds {max} characters"
    );
    ensure!(
        !value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t'),
        "{name} contains terminal control characters"
    );
    Ok(())
}

pub fn identifier(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'),
        "invalid ID {value:?}; use 1–64 lowercase letters, digits, or underscores"
    );
    Ok(())
}

fn strings(values: &[String], name: &str, count: usize, width: usize) -> Result<()> {
    ensure!(values.len() <= count, "too many {name}");
    for value in values {
        text_field(value, name, width)?;
    }
    Ok(())
}

fn unique<'a>(values: impl Iterator<Item = &'a str>, name: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        ensure!(seen.insert(value), "duplicate {name}: {value}");
    }
    Ok(())
}

fn character(value: &Character) -> Result<()> {
    identifier(&value.id)?;
    text_field(&value.name, "character name", 80)?;
    text_field(&value.personality, "personality", 600)?;
    ensure!(value.appearance.age <= 1000, "age must be at most 1000");
    for field in [
        &value.appearance.face,
        &value.appearance.hair,
        &value.appearance.eyes,
        &value.appearance.build,
        &value.appearance.clothing,
    ] {
        text_field(field, "appearance", 400)?;
    }
    strings(&value.appearance.accessories, "accessories", 8, 160)?;
    strings(
        &value.appearance.distinctive_features,
        "distinctive features",
        8,
        160,
    )
}

impl GameState {
    pub fn initial() -> Self {
        serde_json::from_str(include_str!("../prompts/world.json")).expect("bundled starting world")
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "unsupported state schema version");
        text_field(&self.premise, "premise", 2000)?;
        text_field(&self.art_style, "art style", 600)?;
        text_field(&self.summary, "summary", 1800)?;
        ensure!(self.lorebook.len() <= 512, "at most 512 world notes");
        unique(self.lorebook.iter().map(|e| e.id.as_str()), "world note")?;
        for entry in &self.lorebook {
            identifier(&entry.id)?;
            text_field(&entry.text, "world note", 600)?;
            strings(&entry.tags, "world note keywords", 8, 64)?;
        }
        if let Some(note) = &self.director_note {
            text_field(note, "story direction", 600)?;
        }
        for (id, paths) in &self.references {
            ensure!(
                self.characters.contains_key(id) || self.locations.contains_key(id),
                "unknown reference identity"
            );
            strings(paths, "reference paths", 3, 600)?;
        }
        text_field(&self.world.time, "world time", 100)?;
        text_field(&self.world.weather, "weather", 120)?;
        ensure!(
            self.locations.contains_key(&self.world.location),
            "unknown current location"
        );
        ensure!(
            self.characters.contains_key("player"),
            "world requires a player character"
        );
        for (key, c) in &self.characters {
            ensure!(key == &c.id, "character key/id mismatch");
            character(c)?;
        }
        for (key, l) in &self.locations {
            ensure!(key == &l.id, "location key/id mismatch");
            identifier(key)?;
            text_field(&l.name, "location name", 100)?;
            text_field(&l.description, "location description", 800)?;
        }
        for (item, quantity) in &self.inventory {
            identifier(item)?;
            ensure!(
                *quantity > 0 && *quantity <= 9999,
                "invalid inventory quantity"
            );
        }
        for flag in self.flags.keys() {
            identifier(flag)?;
        }
        for r in &self.relationships {
            ensure!(
                self.characters.contains_key(&r.from) && self.characters.contains_key(&r.to),
                "unknown relationship character"
            );
            ensure!(
                r.from != r.to && (-100..=100).contains(&r.trust),
                "invalid relationship"
            );
            text_field(&r.note, "relationship note", 400)?;
        }
        let pairs: Vec<String> = self
            .relationships
            .iter()
            .map(|r| format!("{}:{}", r.from, r.to))
            .collect();
        unique(pairs.iter().map(String::as_str), "relationship")?;
        for (key, thread) in &self.plot_threads {
            identifier(key)?;
            ensure!(key == &thread.id, "plot-thread key/id mismatch");
            text_field(&thread.description, "plot thread", 600)?;
        }
        for (key, objective) in &self.objectives {
            identifier(key)?;
            ensure!(key == &objective.id, "objective key/id mismatch");
            text_field(&objective.description, "objective", 600)?;
        }
        for fact in &self.discovered_facts {
            text_field(fact, "fact", 500)?;
        }
        if let Some(scene) = &self.current_scene {
            self.validate_scene(scene)?;
            ensure!(
                scene.location == self.world.location && scene.time == self.world.time,
                "scene/world mismatch"
            );
        }
        Ok(())
    }

    fn validate_scene(&self, scene: &Scene) -> Result<()> {
        ensure!(
            self.locations.contains_key(&scene.location),
            "scene uses unknown location {}",
            scene.location
        );
        text_field(&scene.time, "scene time", 100)?;
        for field in [&scene.mood, &scene.lighting, &scene.camera] {
            text_field(field, "scene descriptor", 240)?;
        }
        ensure!(
            scene.visual_importance.is_finite() && (0.0..=1.0).contains(&scene.visual_importance),
            "visual_importance must be in [0,1]"
        );
        ensure!(scene.characters.len() <= 8, "at most 8 visible characters");
        unique(
            scene.characters.iter().map(|c| c.id.as_str()),
            "scene character",
        )?;
        for c in &scene.characters {
            ensure!(
                self.characters.contains_key(&c.id),
                "unknown scene character {}",
                c.id
            );
            text_field(&c.emotion, "emotion", 160)?;
            text_field(&c.pose, "pose", 240)?;
        }
        strings(&scene.actions, "scene actions", 6, 400)
    }

    /// Work on a clone. The caller commits only the fully validated result.
    pub fn apply(&self, output: &TurnOutput) -> Result<Self> {
        text_field(&output.narration, "narration", 5000)?;
        ensure!(
            (2..=5).contains(&output.choices.len()),
            "provide 2–5 choices"
        );
        strings(&output.choices, "choices", 5, 240)?;
        unique(output.choices.iter().map(String::as_str), "choice")?;
        ensure!(output.dialogue.len() <= 8, "at most 8 dialogue lines");
        let changes = &output.state_changes;
        for (name, len, limit) in [
            ("new characters", changes.new_characters.len(), 3),
            ("new locations", changes.new_locations.len(), 3),
            ("inventory changes", changes.inventory.len(), 12),
            ("flag changes", changes.flags.len(), 12),
            ("relationship changes", changes.relationships.len(), 8),
            ("plot-thread changes", changes.plot_threads.len(), 8),
            ("objective changes", changes.objectives.len(), 8),
            ("memories", output.memory_updates.len(), 5),
        ] {
            ensure!(len <= limit, "too many {name}");
        }
        let mut next = self.clone();
        for c in &changes.new_characters {
            character(c)?;
            ensure!(
                !next.characters.contains_key(&c.id),
                "cannot replace canonical character {}",
                c.id
            );
            ensure!(
                !next
                    .characters
                    .values()
                    .any(|old| old.name.eq_ignore_ascii_case(&c.name)),
                "character name already exists: {}",
                c.name
            );
            next.characters.insert(c.id.clone(), c.clone());
        }
        for l in &changes.new_locations {
            ensure!(
                !next.locations.contains_key(&l.id),
                "cannot replace canonical location {}",
                l.id
            );
            next.locations.insert(l.id.clone(), l.clone());
        }
        next.validate_scene(&output.scene)?;
        for d in &output.dialogue {
            ensure!(
                d.speaker != "player",
                "dialogue is NPC speech only; do not invent player dialogue"
            );
            ensure!(
                output.scene.characters.iter().any(|c| c.id == d.speaker),
                "speaker {} is not in the scene",
                d.speaker
            );
            text_field(&d.text, "dialogue", 1200)?;
        }
        unique(
            changes.inventory.iter().map(|v| v.item.as_str()),
            "inventory operation",
        )?;
        for change in &changes.inventory {
            identifier(&change.item)?;
            ensure!(
                change.delta != 0 && (-100..=100).contains(&change.delta),
                "inventory delta must be nonzero and within -100..100"
            );
            let old = i64::from(*next.inventory.get(&change.item).unwrap_or(&0));
            let value = old + i64::from(change.delta);
            ensure!(
                (0..=9999).contains(&value),
                "inventory underflow/overflow for {}",
                change.item
            );
            if value == 0 {
                next.inventory.remove(&change.item);
            } else {
                next.inventory.insert(change.item.clone(), value as u32);
            }
        }
        unique(
            changes.flags.iter().map(|v| v.flag.as_str()),
            "flag operation",
        )?;
        for flag in &changes.flags {
            identifier(&flag.flag)?;
            next.flags.insert(flag.flag.clone(), flag.value);
        }
        let pairs: Vec<String> = changes
            .relationships
            .iter()
            .map(|r| format!("{}:{}", r.from, r.to))
            .collect();
        unique(pairs.iter().map(String::as_str), "relationship operation")?;
        for r in &changes.relationships {
            next.relationships
                .retain(|old| old.from != r.from || old.to != r.to);
            next.relationships.push(r.clone());
        }
        unique(
            changes.plot_threads.iter().map(|v| v.id.as_str()),
            "plot-thread operation",
        )?;
        for t in &changes.plot_threads {
            next.plot_threads.insert(t.id.clone(), t.clone());
        }
        unique(
            changes.objectives.iter().map(|v| v.id.as_str()),
            "objective operation",
        )?;
        for o in &changes.objectives {
            next.objectives.insert(o.id.clone(), o.clone());
        }
        strings(&changes.discovered_facts, "discovered facts", 8, 500)?;
        for fact in &changes.discovered_facts {
            if !next.discovered_facts.contains(fact) {
                next.discovered_facts.push(fact.clone());
            }
        }
        if let Some(weather) = &changes.weather {
            text_field(weather, "weather", 120)?;
            next.world.weather = weather.clone();
        }
        if let Some(summary) = &changes.summary {
            text_field(summary, "summary", 1800)?;
            next.summary = summary.clone();
        }
        if (self.turn + 1).is_multiple_of(4) {
            ensure!(
                changes.summary.is_some(),
                "refresh the long-term summary on every fourth turn"
            );
        }
        for memory in &output.memory_updates {
            text_field(&memory.text, "memory", 600)?;
            ensure!(
                memory.importance.is_finite() && (0.0..=1.0).contains(&memory.importance),
                "memory importance must be in [0,1]"
            );
            ensure!(memory.tags.len() <= 8, "too many memory tags");
            for tag in &memory.tags {
                identifier(tag)?;
            }
        }
        next.turn = self
            .turn
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("turn counter overflow"))?;
        next.world.location = output.scene.location.clone();
        next.world.time = output.scene.time.clone();
        next.current_scene = Some(output.scene.clone());
        next.validate()?;
        Ok(next)
    }
}

pub fn parse_turn(raw: &str) -> Result<TurnOutput> {
    ensure!(raw.len() <= 96 * 1024, "model response exceeds 96 KiB");
    let raw = raw.trim();
    // Tolerate exactly one fenced JSON document, never scrape arbitrary prose.
    let json = if let Some(body) = raw
        .strip_prefix("```json")
        .or_else(|| raw.strip_prefix("```"))
    {
        body.trim()
            .strip_suffix("```")
            .ok_or_else(|| anyhow::anyhow!("unclosed JSON fence"))?
            .trim()
    } else {
        raw
    };
    if !json.starts_with('{') {
        bail!("response must be one JSON object");
    }
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let output = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        anyhow::anyhow!("invalid output at {}: {}", error.path(), error.inner())
    })?;
    // Deserializing one value alone does not reject a second trailing document.
    deserializer.end()?;
    Ok(output)
}
