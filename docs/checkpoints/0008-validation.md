# Current validation — checkpoint 0008

The three requested model replacements are installed and verified on the M4 / 16 GB Mac. Hornbill uses the exact BennyDaBall Q8_0 encoder with Z-Image-Turbo, shoemoney Gemma 12B Q6 and mlx-community Gemma E4B 8-bit. The default remains 12B. See [runtime pins and provenance](RUNTIMES.md). The original GGUF instruction-model fallback remains installed.

All **45 automated tests pass**: 30 Rust and 15 Python. Formatting, Clippy and the release build pass. Checks cover state transactions, process cleanup, the actual constrained-decoder grammar, legacy image-setting migration, incomplete model receipts, weight corruption, lossless Q8 repacking, non-finite scales, unknown encoder tensors and overwrite protection. [Recorded checks](measurements/model-replacement-checks.txt).

| Current workload | Measured time | Peak MLX | Result |
|---|---:|---:|---|
| Gemma 12B Q6, four turns | 188.47 s/saved turn | 9.84 GiB | 4/4 first attempts; summary refreshed |
| Gemma E4B 8-bit, four turns | 85.58 s/saved turn | 7.85 GiB | 3/4 first attempts; summary refreshed |
| Z-Image text-to-image, 512×768, 9 steps | 126.39 s | 5.57 GiB | PNG and visual inspection passed |
| Z-Image GUI with reference, 512×768 | 55.13 s | 5.57 GiB | Saved and displayed at existing turn 4 |

Story times include every attempt for each saved turn, including E4B's rejected first attempt. Accepted E4B workers alone averaged 68.66 seconds. Decode throughput averaged 6.80 tokens/s for 12B and 14.65 for E4B. These are four-turn samples with cold workers, not statistical performance guarantees. The GUI reference image uses four denoise updates from a nine-step schedule at strength 0.6; it is not comparable to full text-to-image.

## Memory and latency limits

12B peaked at 9.84 GiB of MLX allocations and 7.18 GiB RSS; E4B peaked at 7.85 GiB MLX and 7.35 GiB RSS. These measures overlap and must not be added. During the four accepted 12B attempts, system swap rose 1.91–2.44 GiB per attempt; during accepted E4B attempts it rose 0.51–0.79 GiB. The counter covers the whole Mac, so attribution is not isolated. Neither profile is qualified as swap-free on this 16 GB machine. The exact requested quantizations were retained.

Grammar construction still costs about 25–31 seconds per worker. Prompt lengths reached 4,214 tokens for 12B and 3,928 for E4B, within the 6,144-token context including the 1,536-token output allowance. Every worker exited before the next model began. The next performance milestone is safe reuse of decoder preparation and an isolated memory-pressure comparison; it must not leave both models resident.

## Reliability and story quality

The first 12B attempt and its repair returned zero inventory changes for unchanged items. Rust rejected both without changing the save. The prompt now explicitly uses an empty inventory-change array when nothing was gained or lost; the decoder enforces nonzero integer deltas in -100..100. Its real four-turn rerun passed all first attempts. The [rejected audit](measurements/gemma-q6-inventory-rejected.json) remains alongside the [successful 12B audit](measurements/selected-gemma-12b.json).

E4B's first response gave Mira dialogue but omitted her from the scene cast. Rust rejected it; the built-in repair succeeded, and the next three first attempts passed. [Complete E4B audit](measurements/selected-gemma-e4b.json). No invalid attempt changed canonical state.

Both profiles preserve typed state and produce a summary, but their prose still needs review. 12B's turn-four choices suggest taking the maintenance path after already reaching its entrance. E4B repeats rain starting on successive turns, describes moving along a path while retaining the observatory location, and adds an unsupported claim that the pair have not been followed to its summary. E4B's dialogue often avoids the player's specific question. Valid JSON does not establish narrative truth. The small sample favors 12B for detail, with a substantial speed/memory cost.

## Image and graphical checks

The selected Q8_0 encoder was SHA256-verified and repacked without requantization. All 398 tensor shapes were checked; actual MLX and GGUF dequantizers matched the first and last rows of all 253 matrices exactly. The initial black image exposed left padding in the selected tokenizer. The worker now right-pads and rejects non-finite features, latents and pixels before saving.

Text-to-image and reference-guided PNGs were visually inspected. The earlier reference run's 260.66-second wall time includes an iCloud file-read stall; retain it as failure evidence, not clean latency. A first GUI attempt also failed while reading an offloaded renderer shard. That exact shard was restored from the pinned source and SHA256-verified; both story and image workers now reject macOS offloaded model placeholders before opening them. The final GUI retry used the restored runtime and passed save/display checks. Composition and clothing continuity are observed; exact face identity is unproved.

The current GUI shows both new Gemma labels and Z-Image. Legacy preferences migrated to nine steps while preserving 12B, Smart scheduling, 512×768 and seed 42. The review save `story_1790681920698` remains at turn 4 with the same canonical story state, notes and Mira reference; a new illustration was added. A settings write during an external benchmark was safely refused by the shared lock, then succeeded after the benchmark exited. Prior cancellation/resume, state authorization and stale-turn checks remain documented in [historical validation](checkpoints/0005-validation.md).

The export API works. The in-app browser's export button produced no console error, but its download event timed out, so the actual browser download remains unverified. A separate local review export is provided with this checkpoint. Clean-machine installation, sustained sessions, larger image sizes, the current bounded grammar through GGUF, other hardware and native Tauri packaging remain unqualified.

## Runtime recovery and approved cleanup

iCloud offloaded Python, package and Rust support files under Documents, causing blocked reads. The project was marked Keep Downloaded. Exact Python 3.12.14 and the pinned story/image environments were restored locally, and 30 offloaded Rust files were restored from SHA256-verified Rust 1.98.1 archives. Original environments and offloaded originals remain under `work/runtime/*-icloud-original` and `work/icloud-recovery`; no global tools were changed.

With explicit user approval, superseded Qwen 2.1 and original Gemma 12B/E4B download directories were removed only after each replacement passed. Their combined logical size was 22,232,645,255 bytes (20.71 GiB); this is not the amount of local space reclaimed because some files were already offloaded. Saves, generated images, source, audits and the 7,121,861,440-byte GGUF fallback remain. [Cleanup receipt](measurements/model-cleanup.json).

Model-comparison [summary](measurements/selected-gemma-comparison.json), [CSV](measurements/selected-gemma-comparison.csv), [GUI image](measurements/z-image-gui.json), [conversion](measurements/z-image-conversion.json) and previous failures remain inspectable. Older Qwen and Gemma 4-bit measurements are historical and do not qualify these replacements.
