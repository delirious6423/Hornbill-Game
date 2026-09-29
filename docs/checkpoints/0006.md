# Project Hornbill — Current handoff

# Checkpoint 0006 — Model replacement in progress

Recorded: 2026-09-29T15:46:09.016563+00:00 · 23:46:09 +08. Trigger: context compaction recovery and new user model selections. Exact context-percentage telemetry is unavailable; this is a conservative checkpoint, not an automatic 85% hook.

## Current scope and decisions

The full Hornbill goal is active again according to the goal tool. The user authorized replacement of Qwen Image 2.1 for licensing reasons and explicitly chose BennyDaBall/Qwen3-4b-Z-Image-Turbo-AbliteratedV1, file Z-Image-AbliteratedV1.Q8_0.gguf. This file is a text encoder, not a complete renderer; use it with Z-Image-Turbo. Do not substitute another quantization. The user also replaced the tentative lemuralabs 12B 8-bit candidate with shoemoney/Gemma-4-12B-Abliterated-MLX-q6 and selected mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit for E4B. These latest decisions supersede checkpoint 0005’s review pause and older model defaults.

The current request includes updating this Drive worklog and the GitHub repository. Source ZIP uploads are already authorized. Preserve saves and historical evidence; keep weights, runtime installations and credentials local. Gemma 12B remains the preferred profile, E4B an alternative, and GGUF the approved platform fallback.

## Completed and unverified work

The pinned Z-Image renderer mflux-community/z-image-turbo-mflux-q4 revision f427e257d8e6ffa03edd4d9ac554a05809da456c has downloaded and its weight hashes were verified. The exact selected encoder at revision ce497d288a7ddfd5d0f337c7139349d5d0236bfa has downloaded (4,280,405,248 bytes; expected SHA256 6272f0f8db9e91f5ed748c51da27f646ae2d2cb2a28460a38294192288f85298); installer hash verification and conversion remain to run. The model cards declare Apache 2.0; a pinned upstream Z-Image Apache license is included. The earlier BF16 encoder download was cancelled and only its incomplete temporary file removed.

New local files in workers/z_image include models.json, model.py, worker.py, requirements.in, an unfinished requirements.lock and APACHE-2.0.txt. The converter is intended to preserve the selected GGUF Q8 values in MLX storage without requantization and compare actual dequantized samples. The worker stages encoder, transformer and VAE separately. Both Python files compile syntactically, but conversion, inference and app integration are UNVERIFIED. scripts/image.sh points to this new worker; Rust/UI still point to Qwen. Do not publish or describe this intermediate source as a working replacement.

The existing checkpoint 0005 build passed 35 tests and real story/GUI/reference checks. Those image metrics describe the old Qwen backend and do not qualify Z-Image. The new Gemma selections have only been inspected at their model cards; configuration, strict loader compatibility, download hashes, constrained generation, memory and performance tests are pending. The discarded lemuralabs candidate was researched but never downloaded.

## Locations and active state

Workspace: /Users/yo15m4/Documents/Codex/2026-09-29/project-hornbill. Source: outputs/hornbill. Runtime: work/runtime, selected by .runtime-path. Reuse image-venv (MLX 0.32.3, MFLUX 2924d0c7cd7104a1ab2f18f40d3bedcf47ba8b9c); gguf 0.19.0 has been added and the lock needs refreshing. New image assets: work/runtime/models/z-image-turbo-benny. Selected Gemma research: work/gemma-replacement and work/gemma-replacement-research.log (metadata-only script, last exec handle 39779). The exact Q8 download completed, with no downloader process remaining. Old execution handles 8670 and 47315 expired across compaction; their disk logs establish completion, so do not restart them blindly.

The old live app remains PID 24985 on localhost port 56643, log work/ui-server.log. The private UI capability is only in data/ui-session.json; never publish it. Review save story_1790681920698, The Observatory Signal, remains turn 4 with Mira’s reference; preserve it, the duplicate turn-0 save and all audits. No inference worker was running at the process check. Restart the idle server after the new build is verified. Approximately 15 GiB disk space was available before Q8 conversion, so installing both additional Gemma selections may need storage cleanup or another destination; preserve old weights until that is resolved.

GitHub: delirious6423/Hornbill-Game, private main. Last verified HEAD 17134ebf17449c4cf5fe3a9bada757490970c998, tree 7d4c86cad06583cc7e13b95cadb25dfa5deb50ff. The local source is not a Git checkout; use the connected GitHub API and fresh remote HEAD. Checkpoint 0005 source archive has 87 files, 432468 bytes, SHA256 9c9efd1c06cb910024d6f142d7d41675964b310f8791e8e09ff0351ee0c81fee. External delivery receipts remain in outputs/. Reuse Drive folder 1iD8uZPljGhpJ9rxCHh2BcP65v7-IYang, worklog 1lh94PinjOJo49JXDGrPLpY80sHcHgblqT3oSGkq0amE tab t.0, ZIP file 1fN4zxkI54L-TVCblj6dJ-UF6KWnC3h5U. This checkpoint is an in-progress continuity update; the backup ZIP still represents the last verified build until refreshed.

## Ordered next actions and limits

1. Finish model metadata inspection and pin the two latest Gemma revisions/files. Resolve storage without deleting saves or unrelated files. Do not download the superseded lemuralabs model. Preserve the strict schema and sequential worker lease.

2. Verify and repack the exact Q8_0 encoder, lock dependencies and integrate Z-Image into Rust/UI with nine-step defaults and a compatible saved-preference migration. Test the converter and protocol, then real text-to-image and reference generation with measured memory and visual inspection.

3. Install and qualify the requested Gemma 12B Q6 and E4B 8-bit against the existing constrained story contract. The 12B card uses gemma4_unified/mlx-vlm; inspect installed support before selecting a loader. Do not silently disable strict weight checks or claim fit from file size alone.

4. Update current README, runtime/license notes, validation and handoff; retain historical measurements. Verify tests/build and the idle app restart, then publish the verified source, Drive worklog and refreshed source ZIP. The full goal still has grammar-construction latency, system-swap attribution, export/download and finalization work; do not mark it complete prematurely.
