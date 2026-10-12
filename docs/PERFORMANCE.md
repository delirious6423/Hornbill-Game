# Performance

Measured on one Apple Silicon Mac with the pinned models in [MODELS.md](MODELS.md), other apps open. Treat these as order-of-magnitude figures, not guarantees.

## Measurements

| Workload | Time | Peak MLX | Notes |
|---|---:|---:|---|
| Gemma 12B (Q6), 4 cold turns | 112 s per turn | 9.84 GiB | 4/4 accepted on first attempt; about 9 tokens/s |
| Gemma E4B (8-bit), 4 cold turns | 46 s per turn | 7.85 GiB | 3/4 first attempts, one repair; about 17 tokens/s |
| Image model, 512×768, 9 steps | 126 s | 5.57 GiB | Text only |
| Image model, 512×768, with reference image | 55 s | 5.57 GiB | Last 4 of 9 steps |
| GGUF fallback, one Metal turn | 127 s | n/a | About 10 tokens/s, 7.95 GiB process memory |

Turn times include all attempts. Constrained-decoder setup takes about 1.3 s per turn. Peak process memory was about 5.6 GiB for both Gemma profiles. MLX peak and process memory overlap; do not add them.

## Known limits

- Gemma 12B (Q6) raised swap use and briefly reached warning memory pressure on the test machine. Swap-free operation on 16 GB is not established. Gemma E4B (8-bit) is faster and lighter but less consistent.
- Narrative consistency is probabilistic. Both story models sometimes repeat dialogue, invent details or treat weak evidence as proof. Validation protects saved state, not prose.
- A reference image guides composition and clothing, not exact faces. One reference image per scene; no multi-reference identity.
- Tested on Apple Silicon macOS only. The image worker needs MLX. Linux/NVIDIA and Windows are untested.
- Only 384×576, 512×768 and 768×768 illustrations are offered; only 512×768 was timed.
- Quality studies cover at most eight turns of one starting world.
- One worker runs at a time, so a turn with an illustration is the sum of both phases.

## Running the benchmark

Run from a normal macOS Terminal (sandboxed shells hide Metal), with no other worker running.

```sh
./scripts/benchmark.sh --profiles 12b e4b --turns 4   # same world, actions and seed per profile
./scripts/benchmark.sh --profiles gguf --turns 4      # add --cpu for the CPU path (not a Metal figure)
./scripts/verify-local.sh                             # doctor + two real turns + export, in a new story
```

`--turns` accepts 1–4. Each run creates its own story and writes per-profile JSON reports plus `metrics.csv` to `data/bench_<timestamp>/`, with, per attempt: schema preparation, load, time to first token (MLX), tokens per second, worker total, peak process memory, MLX peak, and system swap before and after. Swap counters are system-wide, not per worker. Repeat runs and compare medians. Read the exported responses as well as the numbers.
