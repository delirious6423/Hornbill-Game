# Contributing and agent guide

Hornbill is a local illustrated interactive-story app for Apple Silicon Macs: a Rust and SQLite core, an Axum browser UI, and Python MLX workers for the story model and image model. See [README.md](README.md) for use and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for design.

## Repo map

| Path | Role |
|---|---|
| `src/story_engine.rs`, `src/image_engine.rs` | Turn and illustration pipelines |
| `src/database.rs`, `src/state.rs`, `src/models.rs` | SQLite access, canonical state, validated model output types |
| `src/prompt_builder.rs`, `src/image_prompt.rs` | Context selection; illustration prompt compiler |
| `src/inference/` | Backend traits, worker process supervision, locking |
| `src/ui.rs`, `ui/` | Local server and embedded front end |
| `src/model_profiles.rs` | Reads `workers/gemma/models.json` |
| `workers/gemma/`, `workers/gguf/`, `workers/z_image/` | Story, fallback and image workers, installers, pins |
| `prompts/` | `world.json`, story instructions, SQL migrations |
| `scripts/` | Setup, checks, diagnostics, benchmark |
| `tests/`, `workers/tests/` | Rust and Python tests |

## Checks

```sh
./scripts/check.sh        # cargo fmt --check, clippy -D warnings, cargo test, Python unittest
```

Run it before handing work back. `./scripts/verify-local.sh` and `./scripts/benchmark.sh` run real models, need a normal macOS Terminal (sandboxes hide Metal) and take minutes; use them only for changes that touch workers or prompts. Tooling comes from `scripts/env.sh` (runtime in `.local/` or the path in `.runtime-path`); use the wrappers rather than global installs.

## Invariants

- SQLite owns canonical state. Models propose; Rust validates and decides. Workers never write to the database.
- Validate before committing, and commit scene, state, memories and audit in one transaction. Failed, cancelled or timed-out attempts must leave the story unchanged.
- Scene and illustration are separate commits. An illustration failure must not affect the scene.
- One worker at a time, under the shared worker lock; every worker exits after one request and its process group is reaped.
- No downloads during play. Downloads happen only in the setup scripts, against pinned revisions with size and hash checks.
- Model pins live in the manifests and scripts listed in [docs/MODELS.md](docs/MODELS.md); change them there and update the doc in the same change.
- UI files are embedded with `include_str!`: rebuild after editing `ui/`.
- Do not change the SQLite schema, the JSON schema, HTTP API paths or CLI flags for wording reasons.
- Never commit `data/`, models, runtime folders or session keys.

## Terminology

Use these terms in UI copy, CLI help, errors, logs and docs. Internal identifiers can keep older names.

| Concept | Use | Not |
|---|---|---|
| Saved playthrough | story ("New story", "Your stories") | save, game (the `--save` flag stays) |
| One accepted action and response | turn | step, round |
| Narrated content of a turn | scene | passage, page, chapter |
| Generated picture | illustration | image, picture, art |
| User-supplied picture guiding an illustration | reference image | visual reference |
| Short tone guidance | story direction | director note |
| User-written world facts (each one a "note") | world notes | lore |
| Alternative starting premise | premise | direction |
| The LLM | story model | language model, LLM |
| The image generator | image model (Z-Image) | renderer, diffusion model |
| Model subprocess | worker | adapter, child, sidecar |
| Model choices | Gemma 12B (Q6), Gemma E4B (8-bit), GGUF fallback | other labels |
| Stopping a job | Cancel, "cancelled" | abort |
| The app | Hornbill | Project Hornbill, the engine |

Style: plain and short, sentence-case headings and buttons, a dialog heading matches the button that opens it, "…" for in-progress text. Error messages say what failed and what to do. Comments explain why, not what. No progress-log language in code or docs.
