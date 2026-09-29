pub mod process;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationSettings {
    pub max_tokens: usize,
    pub context_tokens: usize,
    pub temperature: f32,
    pub seed: u32,
    pub memory_limit_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryRequest {
    pub protocol_version: u32,
    pub messages: Vec<Message>,
    pub schema: Value,
    pub settings: GenerationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendReply {
    pub protocol_version: u32,
    pub raw_text: String,
    pub metadata: Value,
    pub error: Option<String>,
}

#[allow(async_fn_in_trait)]
pub trait StoryBackend {
    /// Returns only once this request's worker has exited and been reaped.
    async fn generate(&mut self, request: &StoryRequest) -> Result<BackendReply>;
}

pub struct DemoBackend;
impl StoryBackend for DemoBackend {
    async fn generate(&mut self, _request: &StoryRequest) -> Result<BackendReply> {
        Ok(BackendReply {
            protocol_version: 1,
            raw_text: include_str!("../../prompts/demo_turn.json").to_owned(),
            metadata: serde_json::json!({"backend":"scripted_demo","is_ai":false}),
            error: None,
        })
    }
}

#[allow(async_fn_in_trait)]
pub trait ImageBackend {
    /// Returns only after the image worker has exited and released its model.
    async fn generate_scene(
        &mut self,
        request: &crate::image_prompt::ImageRequest,
    ) -> Result<BackendReply>;
}
