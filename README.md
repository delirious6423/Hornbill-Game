# Project Hornbill

A fully local illustrated story engine for an Apple Silicon Mac. Enter an action or choose a response, read the next scene, and see its illustration. Rust and SQLite keep your world and saves; Gemma and Z-Image run one at a time in separate processes. The interface is a private local webpage, with no cloud inference or Node/npm runtime.

![Hornbill showing The Observatory Signal at turn four](docs/assets/hornbill-app.jpg)

**Current models:** Gemma 12B Q6 (`shoemoney/Gemma-4-12B-Abliterated-MLX-q6`), Gemma E4B 8-bit (`mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit`), and Z-Image-Turbo with the exact BennyDaBall `Z-Image-AbliteratedV1.Q8_0.gguf` encoder. The image selection is a text encoder paired with a separate Z-Image renderer. Current qualification and limits are in [validation](docs/VALIDATION.md); [runtime notes](docs/RUNTIMES.md) record the exact pins and licenses. Read the [handoff](docs/HANDOFF.md) for the latest checkpoint.

## Open an installed copy

From the source directory in macOS Terminal, run:

```sh
./hornbill ui
```

The launch command opens your browser. Keep that Terminal window running; press Control-C there to stop the app. If Hornbill is already open, use its existing browser tab. The server binds only to `127.0.0.1`, chooses a free port, and creates a private launch link. Its local `data/ui-session.json` can recover the link; it contains a session key, so keep it private. `./hornbill ui --no-open` prints a launch link without opening a browser.

Choose **New story**, then select a choice or type an action. Command-Enter or Control-Enter submits a typed action. Accepted turns save automatically. Select any saved story to resume it. **Cancel** stops the active model; a story already saved before illustration began remains saved.

**Settings** chooses Gemma 12B (Q6, preferred), E4B (8-bit), or GGUF (compatibility), and whether to illustrate at visual moments, every turn, or only on request. **Illustrate now** creates another picture of the current scene. The default image is 512×768, nine steps, seed 42. The UI also offers 384×576 and 768×768; see validation for sizes actually measured.

**Story journal** shows the summary, inventory and unfinished threads. Add a short story direction and world facts, separated by blank lines. Up to four relevant facts join each prompt. A picture can be attached to a canonical character or place. The current low-memory backend uses one picture as a composition reference; it does not lock a character's face. The journal also exports the story and full generation record as JSON.

The starting world is Hornbill Observatory with Alex and Mira. A different premise changes the direction within that world. To create different starting characters and locations, edit a copy of `prompts/world.json` and use the CLI's `new --world` option.

## Current measured behavior

| Current workload | Measured time | Peak MLX | Result |
|---|---:|---:|---|
| Gemma 12B Q6, four turns | 188.47 s/saved turn | 9.84 GiB | 4/4 first attempts; summary refreshed |
| Gemma E4B 8-bit, four turns | 85.58 s/saved turn | 7.85 GiB | 3/4 first attempts; summary refreshed |
| Z-Image text-to-image, 512×768, 9 steps | 126.39 s | 5.57 GiB | PNG and visual inspection passed |
| Z-Image GUI with reference, 512×768 | 55.13 s | 5.57 GiB | Saved and displayed at existing turn 4 |

12B is the preserved default. E4B is faster in this small sample but needed one repair and had weaker story continuity. Both profiles increased system swap; neither is qualified as swap-free on 16 GB. [Validation](docs/VALIDATION.md) records timing boundaries, quality examples and remaining limits. Original Gemma 4-bit/Qwen measurements are retained in [historical validation](docs/checkpoints/0005-validation.md).

## Install a fresh copy

Requires Apple Silicon macOS with Metal, Apple's command-line compiler tools, internet for setup, and sufficient disk space. Budget roughly 45–50 GB for all optional story models, the image stack, environments and build files. Inference itself is offline. Keep the project/runtime fully downloaded: in iCloud-managed Documents, use Finder’s **Keep Downloaded**, or install in a folder outside cloud storage. Offloaded runtime files can stall a worker even though model inference needs no network. Preserve the downloaded model cards, attribution and bundled Apache 2.0 image license.

```sh
# Only if Apple's compiler tools are missing:
xcode-select --install

git clone https://github.com/delirious6423/Hornbill-Game.git
cd Hornbill-Game
./scripts/setup.sh
./scripts/model.sh download 12b
./scripts/model.sh use 12b
./scripts/image.sh setup
./scripts/image.sh download
./scripts/doctor.sh
./scripts/image.sh doctor
./scripts/check.sh
./hornbill ui

# Optional alternative story model and GGUF fallback:
./scripts/model.sh download e4b
./scripts/gguf.sh setup
```

A fresh clone or source ZIP omits `.runtime-path`, so setup uses a `.local` runtime beside the source. For a ZIP, extract it and enter its `hornbill` directory instead of cloning. `HORNBILL_RUNTIME=/absolute/runtime/path ./scripts/setup.sh` selects another location. In the existing development workspace, retain `.runtime-path` to reuse the installed tools and weights in `work/runtime`. Setup never changes your shell profile or requires a permanent model server.

The tools are pinned: Rust 1.98.1; complete Cargo dependencies in `Cargo.lock`; MLX 0.32.3, `mlx-lm` source `3051e26bc72b8ab426af17d14c5599447eb306a5`, Outlines 1.3.3; MFLUX source `2924d0c7cd7104a1ab2f18f40d3bedcf47ba8b9c` in a separate image environment. Exact model revisions, hashes, runtime research and upstream links are in [RUNTIMES.md](docs/RUNTIMES.md), worker manifests and dependency locks. Run Metal commands from an ordinary Terminal; a restricted execution environment may hide the GPU.

## CLI and diagnostics

```sh
# Text-only interactive play; create once, then resume:
./hornbill --save expedition new --title 'The expedition'
./hornbill --save expedition play

# One accepted story turn, followed by an illustration:
./hornbill --save expedition turn --action 'I inspect the radio.'
./hornbill --save expedition illustrate --width 512 --height 768 --steps 9 --seed 42

# Canonical truth and complete audit (export refuses an existing file):
./hornbill --save expedition inspect
./hornbill --save expedition export --output data/expedition-audit.json
./hornbill schema

# A different starting world:
./hornbill --save my_world new --world /absolute/path/world.json

# A clearly labeled scripted demo, no AI required:
./hornbill --save demo new
./hornbill --save demo play --backend demo

# Select the default CLI MLX profile; UI preferences are per story:
./scripts/model.sh use e4b
./scripts/model.sh use 12b

# Explicit CLI settings:
./hornbill --save expedition play --context-tokens 6144 --max-tokens 1536 \
  --temperature 0.6 --seed 42 --memory-gib 10 --timeout-seconds 360 --retries 1
```

In CLI play, enter actions or choice numbers; `/state`, `/help` and `/quit` are available. CLI story commands do not automatically illustrate; the graphical interface coordinates both phases.

The GGUF adapter uses the same engine, schema and saves. It starts a private temporary `llama-server` and stops it before returning:

```sh
./hornbill --save expedition play --backend gguf --model "$(./scripts/gguf.sh path)"

# Explicit CPU fallback is much slower:
./hornbill --save expedition turn --backend gguf --cpu \
  --model "$(./scripts/gguf.sh path)" --timeout-seconds 900 \
  --action 'I listen to the radio.'
```

Pinned fallback: llama.cpp b11247 and Gemma 4 12B Q4_K_M GGUF. On Linux/NVIDIA, supply a suitable llama.cpp build with `--llama-binary` and `--model`; the Rust story engine needs no model-specific change. That platform has not been qualified here. The bundled Z-Image worker currently requires Apple MLX, and process-tree cleanup targets macOS/Linux, not Windows job objects. GGUF is a story-backend fallback, not a claim that every image backend is portable.

## How it works

1. Rust locks the save and selects a bounded context from SQLite.
2. A Gemma worker generates schema-constrained JSON. The decoder enforces list limits and identifier spelling; Rust validates identities, state changes and player agency, with a recorded repair attempt if needed.
3. The worker exits; Rust atomically saves the accepted turn, state and audit.
4. Smart scheduling reuses the previous image or starts the isolated Z-Image worker.
5. Z-Image materializes the text embedding, releases the encoder, denoises, releases the transformer, decodes in tiles, saves a PNG and exits.
6. The interface displays the saved result. A failed image leaves the story and previous image intact.

A shared inherited worker lock prevents Hornbill's story and image models from overlapping. Timeout, cancellation and aborted futures kill their process group. Locks do not govern unrelated AI applications.

Canonical appearance and personality belong to the app. The model selects actions, pose and emotion; an image-prompt compiler injects stable visual identity. Context includes a cumulative summary, selected world state, relationships/objectives/threads, six retrieved memories, up to four lore entries and only two recent scenes with short dialogue excerpts. The complete transcript stays in SQLite. Token counting enforces the configured context limit before inference; oversized input fails clearly instead of silently dropping truth.

Narrative and visual consistency remain probabilistic. JSON validation prevents invalid state mutations, but cannot prove every sentence true or every facial detail consistent. The reference path provides composition guidance; native multi-reference identity editing and long-session quality studies remain outside this MVP.

## Files and verification

```text
src/                 Rust engine, SQLite, local API, image compiler, process supervisor
ui/                  Embedded HTML, CSS and JavaScript; no web build step
workers/gemma/       Offline MLX story adapter and pinned model installer
workers/gguf/        Private llama.cpp adapter and installer
workers/z_image/     Staged Z-Image worker, exact GGUF converter and pinned manifest
prompts/             Starting world, model instructions and SQL migrations
tests/               State, persistence, image and worker-lifecycle regressions
scripts/             Setup, checks, diagnostics and benchmarks
data/                Local SQLite saves, generated PNGs and imported references
docs/                Architecture, protocol, measured evidence and handoff
```

```sh
./scripts/check.sh
./scripts/verify-local.sh
./scripts/benchmark.sh --profiles 12b e4b --turns 4
```

`check.sh` runs formatting, Clippy, Rust regressions and Python protocol tests. Verification/benchmark commands create separate saves and record real inference; they take several minutes. See [BENCHMARKS.md](docs/BENCHMARKS.md). Frontend files are embedded in the Rust binary: rebuild/relaunch after editing them. The launch wrapper builds if needed.

Back up SQLite with its backup API or export the complete record; copying only an active `.sqlite3` file can omit its WAL. PNGs/references stay on disk and need their own local backup. The Drive source ZIP intentionally contains code, documentation and curated test evidence, not live saves, model weights or session credentials.

For the next development session, read [HANDOFF.md](docs/HANDOFF.md), [WORKLOG.md](docs/WORKLOG.md), [VALIDATION.md](docs/VALIDATION.md) and `docs/drive-log.json`. The standing Drive checkpoint policy is in `AGENTS.md`; exact context-percentage telemetry is unavailable, so checkpoints use conservative milestones and compaction boundaries.
