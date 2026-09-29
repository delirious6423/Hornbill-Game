# Project Hornbill — Current handoff

# Checkpoint 0007 — New image backend passes; story runtime repair in progress

Recorded: 2026-09-29T16:17:29.602856+00:00 · 16:17:29 UTC. Trigger: verified image replacement milestone plus an iCloud runtime-file interruption, before further substantial work. Exact 85% context telemetry remains unavailable.

## Scope and latest user decisions

Continue the active Hornbill goal and the current model replacement task. Use exactly BennyDaBall Z-Image-AbliteratedV1.Q8_0.gguf with Z-Image-Turbo, shoemoney/Gemma-4-12B-Abliterated-MLX-q6 for 12B, and mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit for E4B. The earlier lemuralabs candidate is discarded. The user explicitly approved removing only superseded Qwen 2.1 and old Gemma 12B/E4B downloads after each replacement passes. Preserve all saves, images, code, audits and the GGUF fallback. Update both Google Drive and GitHub; source ZIP backup remains authorized.

## Image replacement and code verification

The selected encoder at revision ce497d288a7ddfd5d0f337c7139349d5d0236bfa is downloaded, SHA256 verified and losslessly repacked to MLX Q8 group 32. The first and last rows of all 253 matrices match the GGUF reference dequantizer exactly; all 398 tensors match the expected shapes. Conversion took 31.44 seconds and the native cache totals 5,028,771,857 bytes. The renderer is mflux-community/z-image-turbo-mflux-q4 at f427e257d8e6ffa03edd4d9ac554a05809da456c. No alternate encoder quantization is substituted.

The first integration run produced a black image. Diagnosis found the selected tokenizer defaulted to left padding, while MFLUX expects valid tokens at the start and can produce NaNs for fully masked causal rows. The worker now explicitly right-pads and rejects non-finite features, latents and decoded pixels before writing. The failed image is only work/z-image-failed-black.png; its failed-encoding audit is retained in docs/measurements/z-image-initial-failure.json.

A corrected 512x768, nine-step text-to-image run passed and was visually inspected: 126.39 seconds total, 5.57 GiB MLX peak, 4.74 GiB RSS. System swap decreased in this sample. outputs/Hornbill-Z-Image-Scene.png visibly shows Mira, the radio and rainy observatory setting. The reference-guided result outputs/Hornbill-Z-Image-Reference.png also passed visual inspection and retained the attached composition/clothing. It used four denoise updates (41.79 seconds denoising), with 5.57 GiB MLX peak. Its 260.66-second wall time includes a stalled local bytecode read during PNG saving; do not use that as an uncontaminated inference benchmark. Exact face identity remains unproved.

All 43 tests passed: 30 Rust and 13 Python, including legacy image-setting migration, stale/incomplete model receipts, lossless Q8 block mapping, bad scales, unknown tensor keys, output overwrite protection and same-size weight corruption. Formatting, Clippy and release build passed. New Rust/UI code selects Z-Image and nine steps, preserves old saves/seeds/policies, and reads the shared Gemma manifest. Runtime/license notes, architecture and README are updated locally. This source has not yet been published as the final replacement; the live app is still the old binary.

## Requested Gemma profiles and storage interruption

12B pin: shoemoney/Gemma-4-12B-Abliterated-MLX-q6, revision edd4c19a3edc7d4637c729fcb776e190ba0b7b25, 9,728,621,337 weight bytes, local directory work/runtime/models/12b-abliterated-q6. All files downloaded and hashes verified. E4B pin: mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit, revision db71b2644248b4f9e7e31ede5e6f631b870d8af5, 7,988,669,526 weight bytes, directory e4b-obliterated-8bit. E4B installation has not progressed past runtime imports. Neither new story model has yet passed real generation. Keep strict weight loading; the pinned mlx-lm already supports gemma4_unified text extraction.

macOS/iCloud offloaded runtime files under Documents: the inventory work/offloaded-runtime.json found 10,698 files, about 643 MB across Python, both environments and bytecode caches. Processes blocked in read() on PIL/GifImagePlugin bytecode, Python aliases/collections/enum, tqdm METADATA and pyvenv.cfg. This is a concrete storage failure, not evidence of model memory incompatibility. Finder Keep Downloaded was enabled on the project and verified by cmdUnpin and kept-downloaded badges. Some queued files remain unavailable. No iCloud sharing/account/security setting was changed.

The stalled reference image completed after its cache hydrated. The exact tqdm metadata was restored from the image environment only after verifying its hash/size against the original wheel RECORD. The same Python 3.12 environment configuration was restored from the image environment; offloaded originals remain under work/icloud-recovery. These narrow repairs were insufficient because more standard-library files were offloaded. A clean copy of exact Python 3.12.14 is now being installed under work/runtime/python-recovered using existing uv, with no global Python changes. Log: work/python-runtime-repair.log. Recreate the story environment from the SAME requirements.lock using the restored interpreter; preserve the old environment until replacement checks pass. This repair is necessary; do not redownload already-verified weights.

The first new 12B benchmark was cancelled with SIGINT before generation because imports blocked. Save bench_12b_1790697851956003000 remains turn 0; audit outputs/hornbill/data/bench_20260930_000411/12b.json records interruption. Later restart attempts stalled during Python startup and created no further save. Their benchmark/downloader PIDs 30414 and 30423 were stopped before runtime repair. Old Qwen weights were removed only after both new image tests passed: 10,280,028,048 logical bytes; receipt docs/measurements/model-cleanup.json. Original Gemma weights are still present pending successful replacements.

## Paths, app and publication

Workspace /Users/yo15m4/Documents/Codex/2026-09-29/project-hornbill; source outputs/hornbill; runtime work/runtime; .runtime-path selects it. New image worker/converter is workers/z_image, dependency lock now includes gguf 0.19.0 alongside unchanged MFLUX and MLX pins. Image assets are models/z-image-turbo-benny. Curated evidence is docs/measurements/z-image-conversion.json, z-image-text-to-image.json, z-image-reference.json, z-image-initial-failure.json and model-replacement-checks.txt. Use system /usr/bin/python3 for simple file maintenance while project Python is being repaired; configure its temporary bytecode cache outside the offloaded runtime.

Old app PID 24985 is still on localhost port 56643; its source worker path/weights are superseded, so do not call it a verified current build. Check idle status and restart it on the new binary after story runtime qualification. Review save story_1790681920698 remains turn 4 with Mira’s reference and all prior images/audits. Private session capability remains only data/ui-session.json and must never be uploaded. The preexisting unused turn-0 save and all benchmarks remain intact.

Checkpoint 0006 Drive text, date chip and headings were verified. Its five documentation files were published and all hashes/modes read back at GitHub main commit a8831dca30a5a1f959931a8d6952da29bfe1c2a3, tree bb091b50263df8426d08e8041e2287fe6397df28. Final new source and ZIP are still pending. Use connected GitHub API, fresh remote HEAD and preserve prior commits; local source is not a Git checkout. Reuse Drive folder 1iD8uZPljGhpJ9rxCHh2BcP65v7-IYang, worklog 1lh94PinjOJo49JXDGrPLpY80sHcHgblqT3oSGkq0amE tab t.0 and source ZIP file 1fN4zxkI54L-TVCblj6dJ-UF6KWnC3h5U. The last verified source ZIP remains checkpoint 0005; external receipts identify it.

## Next actions

1. Finish restoring exact Python 3.12.14 and pinned story dependencies, then verify offline imports/doctor. Do not launch duplicate download or inference processes. Preserve the original environments and use the fresh local runtime where needed.

2. Resume the exact E4B download and four-turn new-12B benchmark. Measure both selected story models, including the summary turn, under the existing strict grammar. Record memory/swap and failures honestly. Remove each original Gemma download only after its replacement passes.

3. Select the new 12B CLI profile, restart the idle GUI, verify saved-setting migration and new image generation on the review save without advancing its story. Finalize current validation/review docs and remove inactive old Qwen worker source if appropriate while retaining historical audits.

4. Publish the verified source and worklog to GitHub, refresh the Drive checkpoint and approved ZIP, and verify all receipts. Full-goal grammar latency, swap attribution, browser export-download and finalization remain open until resolved; do not mark the whole project complete prematurely.
