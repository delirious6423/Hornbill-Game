# Project Hornbill — Current handoff

## Checkpoint 0005 — Bounded memory generation; user review point

Recorded: 2026-09-29 22:25:09 MYT (14:25:09 UTC). Trigger: user requested completion of the next small milestone, then a checkpoint/worklog for inspection. They explicitly answered “Pause for my review” to pausing the overall goal after this checkpoint. Exact context-percentage telemetry remains unavailable; no automatic 85% hook is installed.

## Current scope and next-session rule

The original goal is “Finish project hornbill.” This checkpoint completes the smaller requested milestone: stop runaway memory generation at the decoder, then rerun the failed summary turn without losing the save. The user now wants to inspect the build. The overall goal is now confirmed paused. Finish saving and publishing this checkpoint under the user’s new GitHub instruction; do not start another implementation milestone until the user resumes. This is not a claim that every full-MVP performance/finalization concern is resolved.

Keep Gemma 12B preferred, E4B available as the faster option, and GGUF as the authorized platform fallback. Rust/SQLite owns truth and model workers run sequentially. The approved source ZIP may contain code, docs and curated generated test evidence. Weights, runtime installations, live databases and credentials stay local. Reuse the existing Drive folder, worklog and archive file ID. No unanswered user question remains.

## Completed milestone and exact evidence

The live fourth-turn summary test exposed a genuine failure: Gemma repeated memory entries until its 1,536-token reply ended inside the 35th entry. Attempt 20 was rejected; canonical state remained turn 3. The old-schema repair attempt 21 was cancelled and recorded as interrupted. The failed and interrupted audits are preserved in docs/measurements/ui-before-bounded-schema.json.

Added Schemars attributes in src/models.rs so the generation grammar enforces the engine’s existing collection limits, including at most five memories, eight memory tags, bounded choices/scene/state-change lists, and lowercase identifier spelling. This does not change saved state formats or weaken Rust validation. It also prevents the earlier uppercase “KESTREL-7” memory-tag error while allowing that exact proper name in normal prose.

Two regression tests compile the actual Rust-generated schema through Outlines Core. They accept a valid five-memory response, reject the observed 35-entry form, and accept proper names in prose while rejecting an uppercase internal tag. All 35 tests pass: 27 Rust (7 process, 15 story, 5 image) and 8 Python. Formatting, Clippy, release build and JavaScript syntax checks passed. Evidence: workers/tests/test_schema_contract.py and docs/measurements/bounded-schema-checks.txt.

Replayed exactly the failed action through the GUI from unchanged turn 3. Attempt 22 succeeded on its FIRST attempt, committed turn 4, produced one memory and a refreshed summary retaining Dr. Aris and KESTREL-7, and exited its worker. Time 117.98 seconds; 602 generated tokens from 3,626 prompt tokens; decode 11.74 tokens/s. Full evidence: docs/measurements/ui-bounded-schema-rerun.json. The final GUI visibly shows the accepted narration, dialogue, choices and reused illustration.

## Performance issue to investigate next

The stricter grammar took 24.14 seconds to compile in the real 12B rerun. Time to first token was 35.29 seconds. Peak MLX was 7,580,750,734 bytes (7.06 GiB); peak process RSS was 6,106,382,336 bytes (5.69 GiB). Do not sum these different memory measures.

The SYSTEM swap counter increased from 2,496,722,370 to 5,281,939,456 bytes during that run: +2,785,217,086 bytes, about 2.59 GiB. The precise contribution of Hornbill versus other applications is not established. This materially limits any claim that the latest 12B configuration is comfortably swap-free on 16 GB. Preserve the evidence and profile decoder construction/total resident memory before calling performance fully qualified. Consider caching/reducing grammar-construction overhead and comparing bounded-schema E4B, without weakening the memory-list safeguard. These are next steps, not implemented optimizations.

Earlier four-turn model comparison, before this grammar change: 12B averaged 83.70 seconds and 11.15 decode tokens/s, peak MLX 7.04 GiB; E4B 43.08 seconds and 18.07 tokens/s, peak 4.38 GiB. Both passed four first attempts and summary refresh in that earlier sample. Do not treat that as proof of universal first-attempt acceptance or the new grammar’s latency.

## Other functionality already exercised

A private Rust/Axum browser interface is implemented with new/resumed stories, narration/dialogue/choices, typed actions, generated pictures, progress/cancel, story journal, settings, lore/director notes, reference attachment and JSON export. It is a localhost graphical MVP, not a packaged Tauri application. The export API works; the browser’s export-download interaction is still untested.

Real GUI flow: new story, saved E4B preference, saved direction/world note, typed action, automatic story-to-Qwen illustration, a later displayed choice with 12B, manual picture reuse, full server restart/resume, lore persistence and local reference upload. Five API checks passed for authorization, cross-origin rejection, stale-turn rejection and PNG serving. A narrow-layout New story control and progress-start display were fixed. Expected-turn checks run inside story/image leases; cancellation drops and kills the worker group.

Cancellation checks passed both before story commit (turn unchanged) and after story commit while image generation started (completed story and previous picture retained). No inference worker remained afterward. At the latest idle check, the story was turn 4 and no Gemma or Qwen process remained. Evidence: docs/measurements/ui-cancellation-checks.json and ui-milestone-checks.json.

Images: first text-to-image 512×768 result took 103.23 seconds, peak MLX 5.13 GiB. The GUI’s later text-to-image result took 89.30 seconds, peak 5.14 GiB. A reference-guided 512×768 result took 43.34 seconds, peak MLX 5.43 GiB/RSS 4.94 GiB with unchanged system swap. At strength 0.6 it used three updates from the six-step schedule, so its time is not directly comparable to full text-to-image. The result visibly retained the attached composition and Mira’s clothing. This is one composition reference, not proven face locking or multi-reference identity conditioning. Fine face details and long-run consistency remain unproved.

Lore/director fields are app-owned and backward compatible. Retrieval includes up to four relevant notes, selected canonical state, six memories, cumulative summary and only two recent scenes with bounded dialogue. Full transcripts remain in SQLite. Prose truth is probabilistic: the latest summary calls the radio battery low although an earlier line said it was holding steady and might drain. This needs qualitative review; schema validity cannot prove narrative consistency.

## Locations, runtime, saves and review materials

Workspace: /Users/yo15m4/Documents/Codex/2026-09-29/project-hornbill.
Source: outputs/hornbill. GitHub repository: https://github.com/delirious6423/Hornbill-Game, default branch main, private. The user explicitly authorized populating it with current progress. The local source directory is not yet a Git checkout; command-line Git authentication is unavailable, so use the connected GitHub API for publication. Do not overwrite this directory or its local data to clone it. No PR is needed for this initial authorized population. Publication verification is recorded in outputs/Project-Hornbill-GitHub-Receipt.json.
Runtime/tools/weights: work/runtime, selected by source .runtime-path. Use existing wrappers and installed weights; no fresh global installation is needed.
Story environment: work/runtime/venv; MLX 0.32.3, mlx-lm source 3051e26bc72b8ab426af17d14c5599447eb306a5, Outlines 1.3.3. Models: models/12b, models/e4b and models/gguf-12b; active CLI profile 12B. Image environment: work/runtime/image-venv, pinned MFLUX source 2924d0c7cd7104a1ab2f18f40d3bedcf47ba8b9c, model assets models/qwen21. Exact model revisions, hashes and license notes are in docs/RUNTIMES.md and worker manifests/locks. Preserve official Qwen qwen-research terms despite community metadata differences.

Database: outputs/hornbill/data/hornbill.sqlite3, schema 2. Review save: story_1790681920698, titled The Observatory Signal, turn 4. Its settings were restored to 12B + Smart images, 512×768, seed 42. Mira has the imported composition reference. Its generated image attempt 4 is reused on turns 3 and 4. An unused same-title test save story_1790681818741 remains at turn 0; preserve it and all benchmark/failed/CPU saves. Do not delete user or test history during handoff.

Published source commit: 1fe3c364bd4753a3f77470aa583056287fa96c5c. All 87 repository files were verified against local Git blob hashes and executable modes; the existing initial commit was preserved. A final documentation synchronization may advance main; the external GitHub receipt records the final commit. Repository access uses the connected GitHub integration because local command-line Git has no credentials.

User-facing review guide: outputs/Hornbill-Review.md. Screenshot: outputs/Hornbill-App.jpg (1280×1359). Images: outputs/Hornbill-First-Scene.png and Hornbill-Reference-Scene.png. Source README and ARCHITECTURE were rewritten for the current app, with full setup/launch and tested limits.

Live server PID 24985, exec session 32119: work/runtime/target/release/hornbill ui --port 56643 --no-open. It is idle and intentionally left open for inspection. Log: work/ui-server.log. Browser tab 1 is at http://127.0.0.1:56643/ and marked deliverable; viewport override was reset. Its private launch URL is only in data/ui-session.json (mode 0600). Never upload that file or its capability. After a new CUA context, call cua.rewriteDocumentation() before using persistent hornbillTab. Do not kill the server while the user is inspecting it.

The browser’s file-chooser automation stalled for about two hours despite the requested short timeout, then selected the correct project PNG. This was an automation delay between model runs, not measured generation time. Upload and reference generation subsequently passed. Avoid repeating the chooser flow unnecessarily. Some narrow-window semantic clicks were unreliable during automation; keyboard navigation and the final desktop interaction worked.

## Backup and remaining work after user resumes

Drive folder ID: 1iD8uZPljGhpJ9rxCHh2BcP65v7-IYang. Worklog ID: 1lh94PinjOJo49JXDGrPLpY80sHcHgblqT3oSGkq0amE, tab t.0. Source archive file ID: 1fN4zxkI54L-TVCblj6dJ-UF6KWnC3h5U, refreshed in place as Project-Hornbill.zip. Local package: outputs/Project-Hornbill.zip. The older local Phase1 ZIP is retained. The separate outputs/Project-Hornbill-Backup-Receipt.json records the archive checksum and verified upload; keeping that receipt outside the ZIP avoids circular hashes. docs/drive-log.json is the worklog sync authority.

1. Read delivery receipts if verification status is needed: Project-Hornbill-GitHub-Receipt.json and Project-Hornbill-Backup-Receipt.json alongside the source ZIP. GitHub source content is verified; final documentation and Drive backup are finalized around this checkpoint. The overall goal is already paused. Leave the idle app available and wait for the user’s review before feature work.
2. On a later explicit resume, first read this handoff, worklog, validation, drive-log and the user’s inspection feedback. Do not rerun completed model/download checks by default.
3. Next technical milestone: investigate the latest 12B swap increase and 24-second grammar construction, and measure the bounded grammar with E4B. Preserve the new list/tag constraints and old audits.
4. Complete the remaining GUI export-download check, source-package launch review and qualitative story consistency review. Current sizes beyond 512×768, long sessions, new grammar through GGUF, other hardware, native Tauri packaging and true identity conditioning are not qualified.
5. Only mark the original full goal complete after required outstanding performance/finalization work is resolved or explicitly scoped by the user. Continue conservative Drive checkpoints around context milestones/compaction and before handoff; exact 85% telemetry is unavailable.
