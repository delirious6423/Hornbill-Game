# Project Hornbill worklog

## 2026-09-29 — Checkpoint 0001: Phase 1 and Drive continuity

- Trigger: user requested a durable Drive worklog at 85% context; initial checkpoint after context compaction.
- Phase 1 implementation status preserved in HANDOFF.md and VALIDATION.md: 22 checks passed previously, two real GGUF CPU turns persisted, MLX/Metal and final prompt-quality refinements remain unverified.
- Created a dedicated Project Hornbill Drive folder and native Worklog & Session Handoff document. Added persistent project instructions and a local resume guide; copied the original project brief for future sessions.
- The 85% rule is an agent instruction with conservative milestone/handoff fallback. No exact-percent monitor, timer, or trusted lifecycle hook is installed.
- No runtime code changes or model runs were made for this logging task.
- Optional source ZIP upload was rejected by automatic approval review because it was outside the worklog authorization. The user then explicitly answered “Also upload the source ZIP”; the source backup is now authorized. The historical rejection is resolved for this payload.
- Next action: obtain the normal-Terminal Metal verification result, then evaluate current prompts and four-turn quality/memory behavior.
- Native Google Doc readback verified the full checkpoint text, 13 headings, 18 list paragraphs, its date chip and both Drive links. The approved source ZIP upload succeeded.
- Worklog sync status is recorded in drive-log.json. The source ZIP includes these continuity documents; its archive checksum/readback receipt is stored alongside the local ZIP.

## 2026-09-29 — Checkpoint 0002: real MLX story loop

- Trigger: compaction recovery and a successful two-turn Metal milestone; exact context percentage remains unavailable. The active user goal is to finish the full local MVP, not stop at Phase 1.
- Metal access is now approved and verified. Prompt-only runs exposed choices/dialogue schema failures; added path-specific repair errors and pinned Outlines 1.3.3 constrained decoding.
- 23 tests, formatting, Clippy and release build passed. Two real 12B turns passed on first attempt, saved turn 2 and exited workers: 71.86/81.89 seconds, 12.31/11.98 tokens/s, about 7.04 GiB MLX peak; system swap counter unchanged. Full audits saved under docs/measurements.
- E4B download started for a same-workload four-turn comparison. Image/UI work remains unimplemented; a newly available quantized Qwen 2.1 staged loader and 6-step turbo adapter need local qualification.
- Current handoff records exact process, save, source/runtime paths, failures, remaining tasks and source-backup status. Prior handoff preserved in docs/checkpoints/0001.md.
- Drive checkpoint 0002 readback verified full text, headings, its native date chip and preservation of checkpoint 0001. Sync completed.

## 2026-09-29 — Checkpoint 0003: model comparison and real Qwen illustration

- Trigger: completed four-turn 12B/E4B comparison and first real image; conservative context milestone.
- Both story profiles passed all four first attempts and summary refresh. Means: 12B 83.70 s/turn, 11.15 tokens/s, 7.04 GiB MLX peak; E4B 43.08 s/turn, 18.07 tokens/s, 4.38 GiB. Retain 12B as preferred for stronger writing and expose E4B as faster option.
- Implemented ImageBackend, shared isolated process transport, cancellation drop guard, canonical prompt compiler, Smart/Always/Manual reuse policy, SQLite image audit/migration, PNG verification and image CLI. All 31 tests, formatting, Clippy and release build passed.
- Installed and verified pinned full-Q4 Qwen 2.1 and unmerged six-step Viggle LoRA in a separate MFLUX environment. First 512×768 image took 103.23 s, peaked at 5.13 GiB MLX/4.67 GiB RSS, and showed slightly decreasing system swap. Measured encoder/transformer eviction and worker exit. Visually inspected the PNG; visible identity/clothing match, fine face details and long-run consistency remain unproved.
- GUI, user reference workflow/validation and lore/director improvements remain. Full goal stays active. No model or download process remains running.
- Source ZIP checkpoint 0002 was uploaded and verified byte-for-byte earlier in this milestone; refresh for current code after Drive checkpoint readback.
- Drive checkpoint 0003 verified by native readback: full text, headings, date chip and previous history preserved.

## 2026-09-29 — Checkpoint 0004: GUI and bounded lore implemented

- Trigger: immediate recovery after compaction; exact context percentage unavailable.
- Built the private Rust/Axum browser UI, saved-story controls, sequential jobs/cancel, settings, journal, lore/director notes, reference upload and export. Added expected-turn checks inside story/image leases and bounded recent dialogue.
- All 33 tests, formatting, Clippy, release build and JavaScript syntax passed. Five live API authorization/stale-turn/image checks passed. Initial real browser render was inspected; narrow-layout New story fix was built but still needs server restart and verification.
- Full GUI story/image loop, revised prompts, cancel/resume/export and reference generation remain unverified. Goal stays active; checkpoint records the running old server PID, fresh-build restart steps, exact paths, preserved saves and ordered next actions.
- Source ZIP on Drive is verified through checkpoint 0003 and will be refreshed after UI qualification.
- Drive checkpoint 0004 readback verified the complete text, seven headings, its native date chip and preserved prior history. Sync completed.

## 2026-09-29 — Checkpoint 0005: bounded generation and user review

- User requested finishing the next small milestone, saving a checkpoint/worklog, and explicitly pausing the overall goal for review afterward.
- Real GUI story/image, choice/typed action, reference upload/generation, notes, restart/resume, cancellation and image reuse were exercised. Saved review story is turn 4; 12B + Smart preferences restored. The idle local app remains available.
- Found a runaway memory array at summary turn 4; rejected output and cancelled repair preserved turn 3. Added decoder-level collection limits and identifier spelling to the Rust schema, with two tests using the actual Outlines grammar. All 35 tests, formatting, Clippy, release build and JavaScript syntax passed.
- Exact failed action rerun accepted first attempt in 117.98 seconds, generated one memory and preserved Dr. Aris / KESTREL-7 in the refreshed summary. Failed and successful audits remain available.
- Important next-milestone issue: decoder compilation 24.14 seconds and system swap +2.59 GiB during the latest 12B run. Attribution is not yet isolated. Do not call current 16 GB memory performance fully settled.
- Reference render measured 43.34 seconds, peak MLX 5.43 GiB, using three denoising updates. Composition/clothing continuity observed; exact face identity unproved.
- README, architecture, validation and the review guide describe completed behavior, exact paths, process/save state, limitations and next actions. GUI export download and full performance/finalization remain for a later resume.
- The user then authorized populating private GitHub repository delirious6423/Hornbill-Game. Published commit `1fe3c364bd4753a3f77470aa583056287fa96c5c` on main while preserving the initial commit. All 87 files matched local Git blob hashes and executable modes. Added a repository preview and portable review/setup guidance; excluded runtime, weights, live data and credentials. Final documentation synchronization and Drive backup receipts follow this source publication.
- The overall goal is confirmed paused at the user’s request. GitHub delivery is authorized checkpoint work, not resumption of feature development.
- Drive checkpoint 0005 readback verified the complete text, seven headings, native date chip, GitHub link and unchanged prior history. Worklog sync completed. The final source archive and GitHub commit are verified separately in the external delivery receipts, avoiding self-referential checksum changes.
