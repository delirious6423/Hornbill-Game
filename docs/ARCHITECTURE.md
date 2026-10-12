# Architecture

Hornbill is one Rust binary plus short-lived Python workers. Rust owns the story state; workers only turn a request into model output.

## A turn

```text
UI action or CLI choice
  take the turn lease (`data/turn.lock`); check the expected version
  read state from SQLite; build a bounded context
  record the request and a generation attempt
  take the shared worker lock; run the story worker; reap it
  validate the JSON and the state changes
  commit scene, state, memories and audit in one transaction
  release the turn lease
  apply the illustration policy (at visual moments / every turn / only when I ask)
  reuse the previous illustration, or run the image worker
  validate the PNG; save the illustration record
```

The scene and its illustration are separate commits. A failed illustration never rolls back the scene or removes the previous illustration. If another turn lands between the two phases, the illustration version check fails instead of attaching the wrong scene. A stale choice (an old tab, or a CLI turn in between) is rejected before an attempt starts.

An invalid reply is recorded and retried from the unchanged state (`--retries`, default 1). Exhausted retries, worker failure and Cancel leave the story unchanged. On restart, attempts left "running" are marked interrupted; uncertain actions are never replayed.

## State ownership

SQLite is the only source of truth.

| Table | Contents |
|---|---|
| `saves.state_json` | Versioned aggregate: world, characters, locations, appearance and personality, inventory, flags, relationships, objectives, plot threads, facts, summary, world notes, story direction, reference paths |
| `turns` | Accepted actions and scenes, with state before and after |
| `memories` | Tagged facts with importance, for retrieval |
| `generation_attempts` | Prompts, model input, raw replies, settings, metrics, errors |
| `image_attempts`, `scene_images`, `save_preferences` | Illustration prompts, settings, timings, failures, and per-story settings |

The story model may add whole new characters and places but cannot change existing appearance, personality or reference paths. Duplicate names are rejected, and dialogue must come from a character present in the scene. World notes, story direction and reference images are edited only through app endpoints that take the same lease and validate the result.

Transactions use WAL with full synchronization and a version comparison. Export and the UI state view read inside one short read transaction, so a concurrent commit cannot mix old state with a newer scene.

## Context building

Each turn sends a bounded projection of state, not the transcript:

- current world conditions and the premise
- up to 5 characters and 5 locations, 24 inventory items and 24 flags, 8 relationships, 6 open threads, 6 active objectives, 10 facts, ranked by keyword match with the action, the current location and the characters in the scene
- cumulative summary, refreshed on every fourth turn
- 6 retrieved memories (recent and high-importance candidates, keyword and tag scoring; no embeddings)
- the 2 most recent scenes, with short dialogue excerpts
- up to 4 relevant world notes and the story direction

The worker counts real tokenizer tokens. Prompt plus reserved output must fit `--context-tokens` (2048–8192, default 6144); otherwise the turn fails with a clear error rather than dropping state silently. The full transcript stays in SQLite.

## Workers

Backends are traits (`StoryBackend`, `ImageBackend`) in `src/inference/`. The engines do not know MLX or llama.cpp; the CLI and UI choose the executable and arguments.

- Protocol: one JSON request on stdin, one JSON envelope on stdout (protocol version, raw text, metadata, optional error). Diagnostics go to stderr. Requests are bounded before spawning; stdout is capped at 512 KiB and stderr at 256 KiB.
- Story request: messages, the JSON Schema generated from the Rust types, token budget, context budget, temperature, seed, memory limit.
- Image request: the compiled prompt, one optional reference image, output path, size, steps, seed, memory limit.
- Isolation: each worker is a new process in its own process group. It exits after one request, so MLX memory is returned to the OS. Timeout, Cancel and aborted futures kill and reap the whole group.
- Locking: a shared worker lock (`data/worker.lock`) is inherited by the worker, so a new worker cannot start while an old one is alive. Only one Hornbill worker runs at a time. The lock does not cover other applications. Never delete it while a worker runs.
- Constrained output: the MLX worker uses llguidance with the Rust-generated schema (list limits, lowercase identifiers, bounded numbers). The GGUF fallback uses llama.cpp grammar through a private loopback `llama-server` with a random key, started and stopped per request. Rust validates every reply again; the decoder is not trusted.
- Image worker: text encoder, then transformer, then tiled VAE decode, each released before the next loads. It right-pads the encoder input and rejects non-finite values before writing a PNG. One reference image is used as img2img composition guidance.
- No downloads during play; offline flags are enforced. Downloads belong to the setup scripts.

## UI server

Axum serves the embedded HTML, CSS and JavaScript from the same process. There is no Node, npm or remote content.

- Binds `127.0.0.1` on a free port (or `--port`).
- A random 192-bit session key is passed once in the launch link's fragment, removed from the address bar and kept in browser session storage. API calls need it as a bearer header and are rejected if the `Origin` differs. No CORS headers are set.
- CSP limits scripts, styles and connections to self, images to self and blob, and forbids framing. Responses are `no-store` and `nosniff`.
- `data/ui-session.json` (mode 0600) holds the launch link and key.
- One job at a time (writing, illustrating, cancelling). The browser polls `/api/job` and reloads state between phases. The job record is transient; committed state survives a restart.
- Model text is rendered as text nodes, never HTML. Uploaded reference images are resized in the browser, then decoded again with dimension limits, resized and saved to a new file by Rust. Image serving resolves only database assets inside the data directory.

## Data on disk

| Path | Contents |
|---|---|
| `data/hornbill.sqlite3` | All stories (plus `-wal` and `-shm` while open) |
| `data/generations/<story>/`, `data/references/<story>/` | Illustration and reference PNGs referenced from the database |
| `data/turn.lock`, `data/worker.lock`, `data/ui.lock`, `data/ui-session.json` | Runtime files; `ui.lock` allows one UI per data folder, `ui-session.json` holds the session key |
| `.local/` (or `HORNBILL_RUNTIME`) | Toolchains, Python environments, models |

Back up with SQLite's backup API (or the JSON export) plus the PNG and reference files. Copying only the `.sqlite3` file can miss its WAL.
