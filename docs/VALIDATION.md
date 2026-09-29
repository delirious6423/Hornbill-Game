# Current validation — checkpoint 0010

The local MVP is implemented: Rust/SQLite story state, isolated story and image workers, bounded context with summaries/retrieval/lore, canonical appearance/reference pictures, configurable illustrations, saved stories, cancellation, and a private browser interface. Exact selected models remain **shoemoney Gemma 12B Q6**, **mlx-community Gemma E4B 8-bit**, and **Z-Image-Turbo with the exact BennyDaBall Q8_0 encoder**. See [runtime pins](RUNTIMES.md) and the [review guide](REVIEW.md).

**50 automated tests pass: 32 Rust and 18 Python**, plus formatting, Clippy and release build. A clean source copy with 35 freshly installed pinned dependencies passed the same suite, saved/reopened/exported a scripted turn, and replayed 352 constrained tokens on Metal. Installed Rust/Python tools and model files were reused. This is a source/dependency rehearsal on this Mac, not a pristine operating-system installation or other-hardware qualification. [Checks](measurements/final-mvp-checks.txt), [clean-source receipt](measurements/clean-source-rehearsal.json).

## Measured performance

| Workload | Time | Peak MLX | Peak process RSS | Result |
|---|---:|---:|---:|---|
| 12B Q6, four cold turns | 112.22 s/saved turn | 9.84 GiB | 5.60 GiB | 4/4 first attempts |
| E4B 8-bit, four cold turns | 46.47 s/saved turn | 7.85 GiB | 5.52 GiB | 3/4 first attempts; one repair |
| Z-Image, 512×768, 9 steps | 126.39 s | 5.57 GiB | See audit | Text-to-image passed |
| Z-Image with a reference, 512×768 | 55.13 s | 5.57 GiB | 4.59 GiB | Saved and displayed at review turn 4 |

Story times include all attempts; accepted E4B workers alone averaged 38.24 seconds. Decode speed averaged 9.00 and 16.64 tokens/s. Compared with checkpoint 0008, mean saved-turn times were 188.47 and 85.58 seconds. The same model pins, settings and four actions were used, but generated lengths differ: this is a workload comparison, not an identical-token microbenchmark. Image measurements are retained checkpoint-0008 results because that worker did not change; the reference image uses four denoise updates from a nine-step schedule at strength 0.6. [Comparison](measurements/llguidance-gemma-comparison.json), [CSV](measurements/llguidance-gemma-comparison.csv), [image audit](measurements/z-image-gui.json).

The llguidance 1.8.0 decoder prepares in 1.25–1.41 seconds, including tokenizer conversion, versus roughly 25–31 seconds for the previous Outlines index. Pure grammar creation is about 2–4 ms. A proposed Outlines disk cache was rejected: its isolated build reached 6.61 GiB RSS and exceeded the 128 MiB serialization cap. The replacement's isolated decoder probes use about 1.1–1.2 GiB RSS, retain the full Rust schema and reject unsupported grammar warnings. No model or matcher survives its worker. [Probe](measurements/llguidance-schema-probe.json), [rejected cache experiment](measurements/outlines-cache-rejected.json).

## Memory limits

MLX and RSS overlap and must not be added. The 12B run changed whole-system swap by +1.47, +0.37, −0.07 and −0.03 GiB per turn. All five E4B attempts showed declining swap usage. Read-only sampling recorded 51 normal and two warning memory-pressure samples; no critical sample was observed. The warnings occurred during 12B. During the sampled interval, whole-system swap-ins and swap-outs were each about 1.9 GiB. This does not qualify 12B as swap-free or free of noticeable system impact.

The sampler began after the first turn started and sampled every ten seconds. Other apps remained open; readings are not solely attributable to Hornbill and short peaks can be missed. At most one Hornbill inference worker was observed, and lock/lifecycle tests cover serialization and cleanup. E4B is the faster/lighter option in these samples. The user-selected 12B default is preserved; model switching is explicit. [Memory summary](measurements/llguidance-memory-summary.json), [samples](measurements/llguidance-memory-samples.json).

## Persistence, retrieval and narrative limits

12B continued to eight real turns, all accepted on first attempt, with summaries at turns four and eight and unchanged canonical character definitions. The longest prompt was 4,552 tokens; with the 1,536-token output allowance it remained within 6,144. Before/after state forms an unbroken chain. A separate 48-turn scripted regression proves an older low-importance relevant memory remains retrievable after reopening while prompts retain at most six memories and two recent scenes. The scripted test establishes persistence/retrieval behavior, not model quality.

Prose quality remains imperfect. Turn six repeated Mira's dialogue from turn five. Turn eight remembered the route but invented a crowd as the reason for taking it, treated a call sign as proof of the sender, and suggested the main entrance after finding the service door ajar. Revised instructions about current position, evidence and completed actions did not eliminate those weaknesses. E4B was less responsive to questions and sometimes described travel without updating its structured location. Grammar plus Rust validation protects typed state but cannot prove narrative truth. [Eight-turn review](measurements/eight-turn-review.json), [full audit](measurements/llguidance-12b-eight-turn.json).

Export and the UI state view now use one short SQLite read transaction across related queries. A deterministic two-connection regression commits a new turn between reader queries and confirms the reader remains consistent without blocking that writer. Error paths release the read transaction. Existing tests cover invalid/duplicate state updates, stale versions, repair from unchanged state, cancelled/timed-out workers and failed images.

## Graphical and image checks

The app has been rebuilt/restarted and the review tab shows **The Observatory Signal — Turn 4** with its selected-model illustration. Canonical state, notes and Mira reference match the earlier export exactly. Settings preserve 12B, Smart illustrations, 512×768, seed 42 and nine image steps. Browser Export created a 463,058-byte JSON file whose content exactly matches the checkpoint export; the download event observer timed out, but the actual file was verified. [Export receipt](measurements/browser-export.json).

The exact Q8_0 encoder was SHA256-verified and losslessly repacked for MLX without another quantization. All 398 tensor shapes and sampled rows of all 253 matrices were checked. Both text-to-image and reference-guided PNGs were visually inspected. References provide composition/clothing guidance; exact face identity and multiple-reference identity editing are unproved. The image worker right-pads the encoder input and rejects non-finite output. [Conversion](measurements/z-image-conversion.json).

## Scope and remaining qualifications

The graphical MVP uses Axum and embedded local HTML/CSS/JavaScript. This avoids a second application runtime and leaves the engine ready for an optional native shell. Tauri packaging, other operating systems/NVIDIA, larger image sizes, multi-reference face locking and statistical long-story studies are future work. There is no cloud inference. Downloads belong to setup; keep model/runtime files fully downloaded in this iCloud-managed workspace.

The explicitly approved superseded model cleanup is complete: 20.71 GiB logical data removed after verification. This is not a claim about physical disk space reclaimed. Saves, generated images, failed audits, old runtime environments and the original instruction-model GGUF fallback remain. [Cleanup](measurements/model-cleanup.json). The original GGUF is not an abliterated equivalent. Earlier model measurements and runtime recovery history remain in [checkpoint 0008 validation](checkpoints/0008-validation.md).

## Current GGUF fallback check

The retained original instruction-model GGUF fallback passed one current-schema Metal turn in 127.06 seconds, with 10.01 decode tokens/s and 7.95 GiB child-process RSS. Its private server and worker exited. This verifies the Mac adapter smoke path, not an abliterated equivalent, a long-run quality result or another hardware platform. The first attempt could not start because iCloud had offloaded a runtime library. Six affected files were restored from the existing SHA256-verified b11247 archive, preserving the originals under `work/icloud-recovery/llama-b11247-offloaded`. The worker now checks the GGUF, executable and adjacent dylibs before starting. [Successful audit](measurements/gguf-final-smoke.json), [failed audit](measurements/gguf-offloaded-rejected.json), [recovery receipt](measurements/gguf-runtime-recovery.json).

The successful GGUF smoke used explicit DRY repetition controls after two earlier word-loop attempts exhausted their token budgets. Both were rejected without advancing the save. The final scene remains stylistically rough (unnecessary hyphenated prose), so this is compatibility evidence rather than a story-quality recommendation. [Repetition failures](measurements/gguf-repetition-rejected.json).
