use crate::{
    models::Scene,
    state::{GameState, text_field},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImagePolicy {
    #[default]
    Smart,
    Always,
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImageSettings {
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub seed: u32,
    pub memory_limit_bytes: u64,
    pub reference_strength: f32,
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            width: 512,
            height: 768,
            steps: 6,
            seed: 42,
            memory_limit_bytes: 9 * 1024_u64.pow(3),
            reference_strength: 0.6,
        }
    }
}

impl ImageSettings {
    pub fn validate(&self) -> Result<()> {
        for size in [self.width, self.height] {
            ensure!(
                (256..=768).contains(&size) && size % 32 == 0,
                "image sizes must be multiples of 32 between 256 and 768"
            );
        }
        ensure!(
            [6, 20, 40].contains(&self.steps),
            "use 6 turbo steps or 20/40 base-model steps"
        );
        ensure!(
            (4 * 1024_u64.pow(3)..=10 * 1024_u64.pow(3)).contains(&self.memory_limit_bytes),
            "image memory limit must be 4–10 GiB"
        );
        ensure!(
            self.reference_strength.is_finite() && (0.3..=1.0).contains(&self.reference_strength),
            "reference strength must be 0.3–1"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageRequest {
    pub protocol_version: u32,
    pub prompt: String,
    pub visual_key: String,
    pub output_path: String,
    pub reference_path: Option<String>,
    pub settings: ImageSettings,
}

pub fn visual_key(state: &GameState, scene: &Scene) -> Result<String> {
    let ids: BTreeSet<_> = scene.characters.iter().map(|c| &c.id).collect();
    let canonical: Vec<_> = ids.into_iter().map(|id| &state.characters[id]).collect();
    Ok(serde_json::to_string(
        &serde_json::json!({"location":state.locations[&scene.location],
        "characters":canonical,"style":state.art_style,"references":state.references}),
    )?)
}

pub fn render_reason(
    policy: ImagePolicy,
    threshold: f32,
    previous_key: Option<&str>,
    current_key: &str,
    importance: f32,
    force: bool,
) -> Result<Option<&'static str>> {
    ensure!(
        threshold.is_finite() && (0.0..=1.0).contains(&threshold),
        "visual threshold must be 0–1"
    );
    Ok(if force {
        Some("requested")
    } else if policy == ImagePolicy::Manual {
        None
    } else if previous_key.is_none() {
        Some("first_illustration")
    } else if policy == ImagePolicy::Always {
        Some("every_turn")
    } else if previous_key != Some(current_key) {
        Some("location_cast_or_identity_changed")
    } else if importance >= threshold {
        Some("significant_scene")
    } else {
        None
    })
}

pub fn compile(
    state: &GameState,
    assets: &Path,
    output: &Path,
    settings: ImageSettings,
) -> Result<ImageRequest> {
    state.validate()?;
    settings.validate()?;
    let scene = state
        .current_scene
        .as_ref()
        .context("advance the story before illustrating it")?;
    let location = &state.locations[&scene.location];
    let mut prompt = format!(
        "{} illustration. {}: {}. {}. Weather: {}. Mood: {}. Lighting: {}. Camera: {}.\n",
        state.art_style,
        location.name,
        location.description,
        scene.time,
        state.world.weather,
        scene.mood,
        scene.lighting,
        scene.camera
    );
    for c in &scene.characters {
        let canonical = &state.characters[&c.id];
        let a = &canonical.appearance;
        prompt.push_str(&format!("{}: age {}; {}; {}; {} eyes; {}; wearing {}; accessories: {}; distinctive features: {}. Current emotion: {}. Pose: {}.\n",
            canonical.name,a.age,a.face,a.hair,a.eyes,a.build,a.clothing,a.accessories.join(", "),a.distinctive_features.join(", "),c.emotion,c.pose));
    }
    prompt.push_str(&format!("Action: {}. Maintain the described identities and clothing. One coherent scene, no captions, dialogue bubbles or interface elements.",scene.actions.join("; ")));
    text_field(&prompt, "image prompt", 9000)?;
    // One composition anchor is supported by this low-memory img2img runtime.
    // True multi-reference identity conditioning is a distinct backend capability.
    let reference = scene
        .characters
        .iter()
        .map(|c| &c.id)
        .chain(std::iter::once(&scene.location))
        .find_map(|id| state.references.get(id).and_then(|paths| paths.first()));
    let reference_path = reference
        .map(|p| resolve_asset(assets, p))
        .transpose()?
        .map(|p| p.display().to_string());
    Ok(ImageRequest {
        protocol_version: 1,
        prompt,
        visual_key: visual_key(state, scene)?,
        output_path: output.display().to_string(),
        reference_path,
        settings,
    })
}

pub fn resolve_asset(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute()
            && path
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
        "invalid asset path"
    );
    let root = root.canonicalize()?;
    let resolved = root
        .join(path)
        .canonicalize()
        .context("local image file is missing")?;
    ensure!(
        resolved.starts_with(root) && resolved.is_file(),
        "image is outside the save folder"
    );
    Ok(resolved)
}
