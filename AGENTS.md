# Project Hornbill: session continuity

Before substantive work, read `docs/HANDOFF.md`, `docs/WORKLOG.md`, `docs/VALIDATION.md`, and `docs/drive-log.json`. The full user brief is `docs/PROJECT_BRIEF.md`. Later user steering overrides these snapshots.

## Standing user instruction: Drive checkpoints

The user requested a Google Drive worklog in a dedicated project folder whenever context is 85% full, sufficient for another session to resume without friction.

- Reuse the existing [Project Hornbill folder](https://drive.google.com/drive/folders/1iD8uZPljGhpJ9rxCHh2BcP65v7-IYang) and [worklog](https://docs.google.com/document/d/1lh94PinjOJo49JXDGrPLpY80sHcHgblqT3oSGkq0amE).
- When reliable context telemetry indicates at least 85%, checkpoint before further substantial work. Do not substitute account usage, total lifetime token counts, or guesses for current-context usage.
- Exact context-percentage telemetry is not exposed in the current tool set. Until it is available, checkpoint conservatively at substantial milestones, before handoff, and at the first opportunity around compaction. Record the actual trigger, and do not claim an exact 85% automatic trigger is installed.
- After compaction or when resuming, read the saved handoff before continuing. If compaction happened before a checkpoint, save one promptly from the recovered context and local evidence.
- The user already authorized recurring worklog creation/updates in this folder. Do not ask again for routine worklog writes. The user subsequently explicitly approved uploading Project-Hornbill-Phase1.zip to this project folder; its source backup is authorized. Preserve the scope of that approval.
- Read the Google Drive and Google Docs skills when using them. Before editing an existing native Doc, follow the skill's trusted read, inspect live tabs/revision, preserve user edits, append a complete timestamped checkpoint, and verify it by readback.
- Refresh `docs/HANDOFF.md` as the current resume guide and append a concise dated entry to `docs/WORKLOG.md`. Keep `docs/drive-log.json` accurate. Set `sync_pending=true` before a remote write; set false and record the verified timestamp only after successful readback.
- Each checkpoint must include objective/scope, latest user decisions, completed changes, exact paths, verified checks and limits, untested changes, errors/failed approaches, active processes/saves, open questions, and ordered next actions. Distinguish measured facts from assumptions.
- On Drive failure, retain the local checkpoint, leave sync pending, report the exact issue, and retry when access returns. Do not create duplicate folders/documents to evade a failed action.
- The approved source ZIP may include the project source, documentation and bundled test evidence. Keep model weights, runtime installations, live save databases and credentials local unless separately authorized. Log concise technical summaries and locations.

## Implementation guardrails

The story/image/UI loop and bounded-memory generation milestone are verified; performance and finalization remain in progress. Checkpoint 0006 records the user’s later image and story model replacement requests and the goal tool’s active status; proceed with that authorized work. The earlier checkpoint 0005 pause is historical. SQLite owns canonical truth; validate before atomic commits. StoryBackend isolates model details. Retain GGUF as the explicitly approved compatibility fallback. Do not advance to images or Tauri until the local story loop and quality/memory gate pass.

Use the existing project-local runtime via `scripts/env.sh` and provided wrappers. Read `.runtime-path` before setup. No unnecessary downloads or global installations. CPU and scripted-demo results do not establish Metal performance.

Checkpoint 0005 records real GUI/reference checks, a repaired runaway memory list, 35 passing tests and the successful turn-4 rerun. It also records a material 12B swap increase and grammar-construction cost that remain to investigate. Follow the current handoff and later user feedback. Preserve failed prompt-only and historical CPU audits. Follow current HANDOFF.md for the active full-MVP objective.

## GitHub snapshot

The user authorized populating https://github.com/delirious6423/Hornbill-Game (private, main) with current progress. Preserve its existing initial commit. The local source is not a Git checkout and local Git credentials were unavailable; use the connected GitHub API or later authenticated Git setup. Upload source, docs and curated evidence only; exclude model/runtime folders, live saves and session credentials. See the handoff and external GitHub receipt for the published snapshot. The initial publishing request alone did not resume feature development; later explicit model replacement requests authorize the current work.
