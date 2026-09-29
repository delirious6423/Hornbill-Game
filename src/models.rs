use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub age: u16,
    pub face: String,
    pub hair: String,
    pub eyes: String,
    pub build: String,
    pub clothing: String,
    #[schemars(length(max = 8))]
    pub accessories: Vec<String>,
    #[schemars(length(max = 8))]
    pub distinctive_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Character {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub id: String,
    pub name: String,
    pub appearance: Appearance,
    pub personality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Location {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Dialogue {
    /// An NPC character ID present in the scene. Never "player" or a display name.
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneCharacter {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub id: String,
    pub emotion: String,
    pub pose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    /// A location ID from persistent state or new_locations.
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub location: String,
    pub time: String,
    #[schemars(length(max = 8))]
    pub characters: Vec<SceneCharacter>,
    #[schemars(length(max = 6))]
    pub actions: Vec<String>,
    pub mood: String,
    pub lighting: String,
    pub camera: String,
    pub visual_importance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InventoryChange {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub item: String,
    #[schemars(schema_with = "inventory_delta_schema")]
    pub delta: i32,
}

fn inventory_delta_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    // Outlines Core currently ignores numeric minimum/maximum. An explicit enum
    // enforces the same nonzero bounds as State::apply, including for GGUF.
    let values: Vec<i32> = (-100..=100).filter(|value| *value != 0).collect();
    schemars::json_schema!({"type": "integer", "enum": values})
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlagChange {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub flag: String,
    pub value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub from: String,
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub to: String,
    /// Absolute trust value, from -100 to 100.
    pub trust: i32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Open,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlotThread {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub id: String,
    pub description: String,
    pub status: ThreadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    #[schemars(regex(pattern = "^[a-z0-9_]{1,64}$"))]
    pub id: String,
    pub description: String,
    pub status: ObjectiveStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MemoryUpdate {
    pub text: String,
    /// Character, location, or plot-thread IDs for retrieval.
    #[schemars(length(max = 8), inner(regex(pattern = "^[a-z0-9_]{1,64}$")))]
    pub tags: Vec<String>,
    pub importance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StateChanges {
    /// Only genuinely new identities; existing appearances cannot be overwritten.
    #[schemars(length(max = 3))]
    pub new_characters: Vec<Character>,
    #[schemars(length(max = 3))]
    pub new_locations: Vec<Location>,
    /// Only items actually gained or lost this turn. Use [] if none changed.
    #[schemars(length(max = 12))]
    pub inventory: Vec<InventoryChange>,
    #[schemars(length(max = 12))]
    pub flags: Vec<FlagChange>,
    #[schemars(length(max = 8))]
    pub relationships: Vec<Relationship>,
    #[schemars(length(max = 8))]
    pub plot_threads: Vec<PlotThread>,
    #[schemars(length(max = 8))]
    pub objectives: Vec<Objective>,
    #[schemars(length(max = 8))]
    pub discovered_facts: Vec<String>,
    pub weather: Option<String>,
    /// A replacement concise summary, preserving unresolved events and commitments.
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurnOutput {
    pub narration: String,
    #[schemars(length(max = 8))]
    pub dialogue: Vec<Dialogue>,
    /// Plain action sentences as strings, never objects, IDs, or next_action maps.
    #[schemars(length(min = 2, max = 5))]
    pub choices: Vec<String>,
    pub state_changes: StateChanges,
    pub scene: Scene,
    #[schemars(length(max = 5))]
    pub memory_updates: Vec<MemoryUpdate>,
}

pub fn output_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(TurnOutput)).expect("schema serialization")
}
