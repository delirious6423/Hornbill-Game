# Application contracts

## Sequential turn boundary

```text
Local UI action / CLI choice
  → acquire per-database turn lease and check expected version
  → read canonical state; select context and retrieve memories/lore
  → record exact story request and generation attempt
  → acquire shared project worker lease
  → run one Gemma worker; receive envelope; reap its process group
  → validate JSON and semantic state changes
  → atomically commit story, state, memories and accepted audit
  → release story turn lease
  → acquire image turn lease and recheck expected version
  → compile canonical image prompt; apply Smart/Always/Manual policy
  → reuse an image, or acquire the same worker lease and run Z-Image
  → validate PNG and save image association; worker exits
  → display the saved result
```

The story and image are intentionally separate commits. An image failure never rolls back accepted narrative or destroys the previous valid image. A CLI turn that happens between the phases makes the image version check fail instead of attaching the wrong scene. A stale UI choice is rejected inside the story lease before an attempt is started.

Invalid story replies are audited and repaired from the unchanged state. Exhausted retries, runtime failure and cancellation cannot advance the world. SQLite transactions use WAL, full synchronization and a version comparison. On restart, abandoned running attempts become interrupted under the turn lease; uncertain actions are never automatically replayed.

The inherited worker lease prevents a new Hornbill model from starting while a surviving old worker holds it. Handled cancellation, timeout and future abortion kill/reap the Unix process group, including descendants. If the OS kills the app abruptly, inspect the audit on restart. Never delete an active lock file. The lock applies to Hornbill, not unrelated model applications.

## Replaceable backends

`StoryBackend::generate` and `ImageBackend::generate_scene` accept typed requests and return a `BackendReply`. The engines know neither MLX nor llama.cpp. The CLI/UI composition boundary selects executables and arguments. Both adapters use one bounded JSON document on stdin/stdout; diagnostics go to stderr or metadata.

Story settings include a token budget, context budget, temperature, seed and memory ceiling. The request includes messages and the schema generated from Rust types. The image request includes the canonical prompt, one optional local reference, output path, dimensions, steps, seed and memory limit. Requests are bounded before spawning; stdout is capped at 512 KiB and stderr at 256 KiB. The reply envelope carries protocol version, raw text, metadata and an optional error. See `src/inference/mod.rs`, `src/image_prompt.rs` and the generated schema for exact fields.

MLX uses Outlines constrained decoding and Rust independently validates the result. The generated schema carries the engine’s collection limits (including at most five memories) and lowercase identifier patterns. This prevents the observed runaway memory-array loop and uppercase memory-tag error during generation itself. Decoder-regression tests compile the real Rust schema with Outlines Core, accepting valid prose/proper names while rejecting both observed invalid forms. GGUF uses llama.cpp grammar generation through a private ephemeral loopback server with a random key and proxies disabled. Offline flags are enforced during inference; dependency/model downloads belong to explicit setup commands. Mac/Linux process supervision is implemented; Windows job-object support remains unqualified.

Z-Image uses separate pinned Python dependencies, the exact selected BennyDaBall Q8_0 GGUF encoder adapted losslessly to MLX storage, and a Q4 Z-Image-Turbo renderer. The encoder is released before denoising and the transformer plus its compiled closure before tiled VAE decoding. The tokenizer is right-padded to match MFLUX's feature slicing and causal mask. Non-finite values are rejected before an image file is written. A fresh worker/process exit gives the final memory-reclamation boundary. One local reference guides img2img composition; true face locking and multiple-reference identity conditioning remain unqualified.

`workers/gemma/models.json` pins the current 12B Q6 and E4B 8-bit selections, file hashes and distinct install directories. The UI and installer use that shared manifest, preventing a stale original-model receipt from masquerading as a replacement. Saved image preferences carry an image-backend version; legacy preferences migrate to nine Z-Image steps without changing story state or other user choices.

## Persistence and audit

`saves.state_json` is a validated canonical aggregate with a version. It contains world conditions, characters/locations, fixed appearance/personality, inventory/flags, relationships, objectives, plot threads, discovered facts, summary, lore/director notes and references. It has no separately writable entity tables that can drift out of sync.

`turns` indexes accepted actions/scenes and before/after state. `memories` indexes bounded tagged facts by recency and importance. `generation_attempts` stores prompts, formatted model input, raw replies, settings, metrics and errors. Image migration 002 adds `image_attempts`, `scene_images` and `save_preferences` without replacing old data. Image attempts include prompt, reference mode, output settings, timings and failures; generated PNGs remain files. Audit export includes both phases and schema version 2.

Lore, director notes and reference associations are application-owned. Their update endpoints take the same save lease and validate the result. Existing identities and reference paths are not model-editable. An imported image is decoded with allocation/dimension bounds, resized and saved to a new file before association. Image serving only resolves successful database assets inside the data directory, rejecting escapes and out-of-root symlinks.

For a live backup use SQLite's backup API, plus the generated/reference files, or a full audit export. A copy of the `.sqlite3` file alone can miss its WAL. Source archives exclude data, runtime, weights and session credentials.

## Bounded context and truth

The model sees current conditions, selected canonical entities, relevant inventory/flags, relationships, active objectives/open threads, selected facts, six retrieved memories, a cumulative summary and two recent scenes. Each recent scene includes at most two short dialogue lines, retaining concrete names while excluding the full narration transcript. Up to four relevant lore entries and a short director note are added. Retrieval uses deterministic keyword/tag scoring, not embeddings.

Every fourth accepted turn must refresh the summary. The worker counts actual tokenizer tokens and requires prompt plus reserved output to fit the configured 2K–8K limit (6,144 default). Dense custom worlds fail clearly if they exceed the budget; shortening descriptions or choosing a larger supported budget is explicit.

The model may introduce complete new character/location records, but cannot replace existing appearance or personality. Duplicate names are rejected. NPC dialogue must come from a character present in the scene; the model cannot speak for the player. State changes are semantically validated before commit. These safeguards do not prove prose-level truth or visual identity, which require separate qualitative evaluation.

## Local interface and tradeoffs

Axum 0.8.9 serves embedded HTML/CSS/JavaScript from the same lightweight Rust process. A browser interface meets the basic graphical MVP without adding Tauri packaging and platform dependencies. The game engine can be hosted by a later native shell unchanged. There is no Node/npm runtime or remote web content.

The server binds only to 127.0.0.1. A random 192-bit session capability is passed once in the launch fragment, removed from the address bar and retained in browser session storage. API calls require its bearer header and reject a different supplied Origin. No permissive CORS is enabled. CSP restricts scripts/styles/connections to self, images to self/blob, and forbids framing. Responses use no-store and nosniff. `data/ui-session.json` is mode 0600 and contains the private recovery link.

One server job runs at a time. Progress tracks writing/illustrating/cancelling; the browser polls status and refreshes committed state between phases. Cancel aborts the task and supervises cleanup. The job record is transient; saved story and attempt records survive restart. Expected-turn checks reject old choices even across tabs/CLI. Per-story preferences choose the backend and image policy; settings never mutate an in-flight request.

UI model text uses text nodes, never HTML interpretation. References are resized in the browser and validated again by Rust. The export button creates a local JSON download. All screenshots and model measurements are documented in `VALIDATION.md`; an API/build check alone is not treated as end-to-end GUI validation.
