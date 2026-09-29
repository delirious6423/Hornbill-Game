# Original user brief

I want you to help me design and implement a fully local interactive AI storytelling application for an Apple Silicon Mac.

My development machine is:

- Apple M4
- 16 GB unified memory
- macOS
- Goal: fully local inference
- Primary language for the application: Rust

The application should work like an interactive visual novel / AI story engine:

1. The user enters an action or chooses an option.
2. A local language model advances the story.
3. The language model outputs structured scene/state information.
4. The application persists the game/world state.
5. A local image-generation model creates an illustration for the scene.
6. The UI shows:
   - generated image
   - narration
   - dialogue
   - choices
7. The user makes another choice and the loop repeats.

My preferred model stack is:

- Story model: Gemma 4 12B IT, ideally 4-bit quantized
- Fallback story model: Gemma 4 E4B if 12B performance or RAM requirements are excessive
- Image model: Qwen Image 2.1, preferably a low-memory 4-bit / distilled / turbo implementation suitable for Apple Silicon
- Apple-native acceleration should be preferred, especially MLX/Metal where practical

IMPORTANT:
Before choosing exact libraries, repositories, checkpoints, quantizations, or APIs, verify what is currently available and maintained. Do not assume package names or model compatibility from older information.

## Core architectural goal

I do NOT want both large models permanently resident in memory.

The application should use sequential inference:

USER INPUT
→ load/run Gemma
→ structured JSON result
→ persist state
→ release/terminate Gemma worker
→ load/run Qwen Image
→ save image
→ release/terminate image worker
→ display result
→ next turn

Because the Mac only has 16 GB unified memory, memory efficiency is one of the highest priorities.

Prefer process isolation for inference workers if that makes memory reclamation more reliable.

The main application should remain lightweight and persistent while inference workers can start and stop independently.

## Preferred software architecture

Use Rust for the persistent application layer.

Potential Rust components:

- Tokio for async/process orchestration
- Serde / serde_json for structured model output
- SQLite for game state and save files
- rusqlite or another appropriate SQLite library
- Axum if a local HTTP/API layer is useful
- Tauri for the eventual desktop UI

However, do not use these automatically if there is a clearly better current alternative. Explain major dependency choices.

The inference backend does not have to be 100% Rust initially.

I am comfortable with an architecture like:

Rust application
├── Gemma worker
└── Qwen Image worker

where one or both workers may temporarily use Python + MLX if that provides significantly better Apple Silicon support.

The Rust application should interact with the workers through a clean abstraction so the backend can later be replaced without changing the game engine.

Possible communication methods include:

- child-process stdin/stdout JSON
- local HTTP
- Unix sockets
- another lightweight IPC mechanism

Prefer the simplest reliable design for the first version.

## Model-runtime strategy

For Gemma, investigate the best current Apple Silicon options, including Rust-native and MLX-based approaches.

Examples worth evaluating may include:

- mistral.rs
- Rust MLX implementations
- mlx-lm / mlx-vlm style runtimes
- any newer better-maintained options

Do not choose solely based on language.

Compare:

- memory usage
- tokens/sec on M4
- model-loading time
- Metal utilization
- structured-output reliability
- quantization support
- Gemma 4 support
- development complexity

For image generation, investigate the most memory-efficient current implementation of Qwen Image 2.1 on a 16 GB M4.

Strongly consider:

- MLX-native implementations
- 4-bit quantization
- distilled/turbo variants
- 4–8 step generation
- staged loading
- component eviction
- tiled VAE decoding
- lower initial resolutions

I care more about interactive latency than maximum image resolution.

Start around something like:

- 512×768
- 640×896
- 768×768

rather than defaulting to 1024×1024.

## Story-engine design

The LLM must NOT be responsible for remembering the entire story through chat history.

The application owns persistent truth.

Use a structured game state containing concepts such as:

- world state
- locations
- current time
- weather
- characters
- character appearance
- character personality
- relationships
- inventory
- flags
- unresolved plot threads
- discovered information
- quests/objectives
- scene history
- summarized long-term memory

The LLM should only receive the relevant subset each turn.

Aim for a relatively small active context, approximately 4K–8K tokens initially.

Do NOT pass the entire story transcript every turn.

Use summarization/state extraction to preserve long-running stories.

## Structured LLM output

The story model should return strongly typed structured data rather than free-form text alone.

Design a Rust schema similar to:

{
  "narration": "...",
  "dialogue": [
    {
      "speaker": "Mira",
      "text": "..."
    }
  ],
  "choices": [
    "...",
    "...",
    "..."
  ],
  "state_changes": {
    ...
  },
  "scene": {
    "location": "...",
    "time": "...",
    "characters": [...],
    "actions": [...],
    "mood": "...",
    "lighting": "...",
    "camera": "...",
    "visual_importance": 0.0
  },
  "memory_updates": [
    ...
  ]
}

Create proper Rust structs/enums for this.

Validate model output before mutating the game state.

Invalid or incomplete model responses should not corrupt the save file.

Design a retry/repair mechanism for malformed structured output.

## Character visual consistency

Do NOT let the LLM freely reinvent character appearances each turn.

Maintain canonical character descriptors in persistent state.

Example:

Mira:
- age
- facial characteristics
- hairstyle
- hair color
- eye color
- body/build where relevant
- clothing
- accessories
- distinctive features

The image-prompt compiler should combine:

canonical character appearance
+
current pose/emotion
+
current scene
+
lighting
+
camera framing
+
global art style

The story LLM should decide things like:

- what a character does
- current emotion
- pose
- scene events

but the application should inject stable visual identity.

If Qwen Image supports useful reference-image conditioning or editing workflows, design the system so reference images can be attached to characters and locations.

Potential layout:

references/
  characters/
    mira.png
    alex.png
  locations/
    observatory.png
    apartment.png

Use reference images where they materially improve continuity.

## Image-generation scheduling

Do not necessarily generate a completely new image after every tiny piece of dialogue.

The story output should contain something like:

visual_importance: 0.0–1.0

or:

new_visual_required: true/false

Generate a new image when appropriate, such as:

- location change
- new character appearance
- major action
- emotional climax
- visual reveal
- costume change
- dramatic event

Otherwise reuse the current image.

However, make this configurable because I may eventually want an image after every turn.

## Persistent game data

Design a save system with SQLite as the source of truth.

Possible entities/tables:

- saves
- story metadata
- characters
- relationships
- locations
- inventory
- world flags
- plot threads
- scenes
- memories
- generated images
- player actions
- model-generation metadata

Store generated images on disk and reference them from the database rather than putting large binary images inside SQLite unless there is a strong reason otherwise.

Each turn should be reproducible/debuggable enough that we can inspect:

- prompt sent to Gemma
- structured response
- state before
- state after
- image prompt
- model/settings used
- seed if applicable
- resulting file

## Project structure

I am imagining something approximately like:

local-story/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── story_engine.rs
│   ├── state.rs
│   ├── database.rs
│   ├── prompt_builder.rs
│   ├── image_prompt.rs
│   ├── inference/
│   │   ├── mod.rs
│   │   ├── llm.rs
│   │   ├── image.rs
│   │   └── process.rs
│   └── models/
│       ├── scene.rs
│       ├── character.rs
│       └── world.rs
├── workers/
│   ├── gemma/
│   └── qwen_image/
├── prompts/
├── data/
├── references/
├── generations/
└── ui/

Feel free to improve this structure.

## Development philosophy

I want an MVP first.

Do not try to build the perfect final architecture before anything runs.

Work incrementally.

### Phase 1

Get a Rust CLI prototype working:

user types action
→ Gemma generates structured scene
→ Rust validates JSON
→ SQLite state updates
→ print narration and choices

No image generation yet.

### Phase 2

Add Qwen Image worker:

story result
→ image prompt compiler
→ image worker
→ generated PNG/WebP

### Phase 3

Add memory optimization:

- worker-process lifecycle
- unload/reload testing
- memory measurements
- model quantization comparisons
- context-size tuning

### Phase 4

Add basic graphical UI.

Tauri is preferred if sensible.

### Phase 5

Improve visual consistency:

- reference images
- character anchors
- location anchors
- seeds
- image-editing/reference-conditioning where supported

### Phase 6

Improve storytelling:

- story director
- state retrieval
- memory summarization
- relationship tracking
- plot-thread retrieval
- lorebook/world knowledge
- scene-specific context selection

## Performance priorities

Optimize in approximately this order:

1. Avoid swapping/paging the system excessively.
2. Keep memory pressure manageable on 16 GB.
3. Reduce image-generation latency.
4. Reduce LLM latency.
5. Improve model-loading/unloading time.
6. Improve image quality.
7. Improve maximum context length.

Measure instead of assuming.

Where practical, report:

- peak process RSS
- MLX/Metal memory if measurable
- model load time
- time to first token
- tokens/sec
- image-generation time
- total turn latency
- swap usage

## Gemma 12B vs smaller model

Start by testing Gemma 4 12B IT at an appropriate 4-bit quantization.

My hypothesis is that the stronger story quality will justify its higher resource usage because it will not coexist with the image model in memory.

But test this rather than assuming it.

If 12B is too slow or causes excessive swap/memory pressure, benchmark the smaller Gemma 4 E4B option.

Compare them specifically for:

- storytelling coherence
- dialogue quality
- character consistency
- instruction following
- structured JSON reliability
- generation speed
- RAM use

Do not compare them only on generic benchmarks.

## Important engineering rule

The Rust game engine should NOT know model-specific implementation details.

Define interfaces/traits conceptually like:

StoryBackend
- generate_turn(...)

ImageBackend
- generate_scene(...)

This should allow us to later replace:

MLX
→ CUDA
→ another inference server
→ a different model

without rewriting the story engine.

I may eventually move this project from the M4 Mac to significantly more powerful NVIDIA hardware, so backend portability matters.

## What I want from you

Act as the technical lead and implementation partner.

Start by doing the following:

1. Verify the current availability and compatibility of the relevant Gemma 4 and Qwen Image 2.1 runtimes for Apple Silicon.
2. Recommend the exact MVP stack.
3. Explain the key tradeoffs briefly.
4. Define the first project structure.
5. Provide the shell commands needed to create the project and install required dependencies.
6. Implement Phase 1.
7. Give me complete files rather than disconnected snippets whenever practical.
8. Keep the code buildable after each stage.
9. Tell me exactly how to run and test each milestone.
10. Do not jump ahead to Tauri/image-generation complexity until the core Rust + Gemma structured-story loop works.

When you encounter uncertain or rapidly changing model/runtime compatibility, verify it rather than guessing.

Prefer working code and measurable tests over theoretical architecture.

Begin with Phase 1 and take ownership of getting the first runnable Rust CLI story loop working on my M4/16 GB Mac.

## Later user instructions

- Project name: Project Hornbill.
- GGUF versions may be used as the platform compatibility fallback.
- Keep a dedicated Google Drive project folder and a worklog around 85% context so a new session can continue without friction.
- The user explicitly approved uploading Project-Hornbill-Phase1.zip to the Drive project folder after the initial optional upload was rejected.

- At checkpoint 0005 the user requested completion of the next small milestone, saving the checkpoint/worklog, then explicitly answered “Pause for my review” before further goal work.

- Latest delivery instruction: upload and populate the user-created private repository https://github.com/delirious6423/Hornbill-Game with current progress. This saves the review checkpoint while implementation remains paused.
