# Current validation — checkpoint 0005

The local graphical story-to-image loop works on the M4 / 16 GB Mac. The latest milestone bounds generated memory lists and identifiers in the actual decoder grammar, then successfully replays the failed fourth-turn summary action. The project is paused for user review; performance and finalization remain open.

35 tests pass: 27 Rust (7 process, 15 story, 5 image) and 8 Python. Formatting, Clippy, the release build and JavaScript syntax checks passed. The two new Python regressions compile the schema emitted by Rust through Outlines Core: five memories are accepted, the observed 35-memory response is rejected, and uppercase proper names remain allowed in prose but not internal tags. See [the recorded checks](measurements/bounded-schema-checks.txt).

| Local workload | Time | Peak MLX | Result |
|---|---:|---:|---|
| Earlier Gemma 12B, four turns | 83.70 s/turn mean | 7.04 GiB | 4/4 first attempts accepted; summary refreshed |
| Earlier Gemma E4B, same actions | 43.08 s/turn mean | 4.38 GiB | 4/4 first attempts accepted; summary refreshed |
| Bounded-schema 12B, failed action replay | 117.98 s | 7.06 GiB | First attempt accepted; turn 4, one memory, refreshed summary |
| Qwen 2.1 Q4 + Viggle, first 512×768 image | 103.23 s | 5.13 GiB | PNG and visual inspection passed |
| Qwen, GUI text-to-image | 89.30 s | 5.14 GiB | Saved and displayed after the story |
| Qwen, reference-guided 512×768 | 43.34 s | 5.43 GiB | Composition/clothing continuity observed |

The reference run used three denoising updates from the six-step schedule at strength 0.6. It is not directly comparable to full text-to-image. Its peak RSS was 4.94 GiB and the system swap counter was unchanged. One composition anchor is supported; face identity, native editing and multiple reference conditioning are not qualified.

## Remaining performance issue

The bounded-schema 12B replay took 24.14 seconds to compile the grammar and 35.29 seconds to first token. It generated 602 tokens from a 3,626-token prompt at 11.74 decode tokens/s. Peak process RSS was 5.69 GiB. MLX allocations and RSS are different measurements and must not be added.

System swap increased from 2,496,722,370 to 5,281,939,456 bytes during this run: **+2.59 GiB**. Hornbill's contribution versus other applications has not been isolated. The latest configuration is therefore not qualified as comfortably swap-free on 16 GB. The next milestone is profiling grammar construction and total memory, then measuring the bounded schema with E4B. Earlier, smaller swap changes cannot override this observation.

## Graphical and persistence checks

- Created and resumed stories, saved model/image preferences, submitted typed actions and a displayed choice, and displayed real narration, dialogue, choices and images.
- Saved direction and world notes, retained them through a full server restart, attached a local reference to Mira and generated a reference-guided picture.
- Verified automatic story-to-image sequencing and Manual image reuse. The review save is `story_1790681920698`, The Observatory Signal, turn 4; preferences are restored to 12B + Smart, 512×768, seed 42.
- Cancelled before story commit without advancing the save, and after story commit while an image started without losing the saved turn or previous picture. No inference worker remained at the final idle check.
- Passed five API checks: unauthorized access rejected, authorized access accepted, foreign origin rejected, stale turn rejected and saved PNG served.
- Fixed and exercised the narrow-layout New story control and immediate progress display. Errors are also shown inside open dialogs.

The original summary attempt repeated memories until the 1,536-token limit cut off entry 35. It was rejected, and its cancelled repair left turn 3 intact. After the schema fix, the exact same action committed turn 4 on its first attempt. SQLite remains canonical, and all failed/interrupted audits are preserved. Existing schema-1 saves migrate without replacement.

These are small sequential local samples, not a statistical quality benchmark. 12B's earlier writing was clearer; E4B was more repetitive. The latest summary describes a low radio battery despite an earlier line saying it was holding steady and might drain. Narrative consistency still needs qualitative review: valid JSON does not establish prose truth.

## Evidence and limits

Full before/after records: [before the constraint fix](measurements/ui-before-bounded-schema.json), [successful replay](measurements/ui-bounded-schema-rerun.json). Other checks: [GUI milestone](measurements/ui-milestone-checks.json), [cancellation](measurements/ui-cancellation-checks.json), [API checks](measurements/ui-api-checks.json), [reference image](measurements/qwen-reference-image.json), and [model comparison](measurements/mlx-model-comparison.csv). Earlier CPU and prompt-only failures remain in `measurements/`.

Still unverified: browser export-download interaction (the export API works), a clean-machine installation, sustained sessions, image sizes beyond 512×768, the new bounded grammar through GGUF, and other hardware. Native Tauri packaging, native Qwen editing and true identity conditioning are not implemented/qualified. Do not restart model tests during the review pause.
