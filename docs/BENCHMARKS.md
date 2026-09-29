# Story-specific benchmark method

Run `./scripts/benchmark.sh --profiles 12b e4b --turns 4` after explicitly downloading both checkpoints. The optional GGUF run is `./scripts/benchmark.sh --profiles gguf --turns 4`. Do not start the runs concurrently. `--cpu` labels the GGUF CPU compatibility path; it is not a Metal performance comparison.

Each model receives the same starting world, actions, seed and generation limits in a separate save. Subsequent worlds can diverge, so this is a narrative-workload comparison rather than an identical-token microbenchmark. Keep the machine's power mode, thermal state, competing apps, and context settings consistent. Repeat complete runs to report medians and ranges, not a single best sample.

## Metrics

| Field | Meaning and boundary |
|---|---|
| `load_ms` | MLX: evaluated weight loading, after tokenizer preparation. GGUF: server launch through healthy/ready. These boundaries differ; compare worker total time for user-visible cost. |
| `ttft_ms` | MLX: start of generation through the first generated token, including prefill. Excludes weight load. GGUF currently uses a nonstreaming completion and records null rather than estimating TTFT. |
| `generation_tps` | Runtime-reported decode throughput; record generated length and prompt tokens alongside it. |
| `worker_total_ms` | Parent spawn through worker exit/reap, including imports, loading, generation and shutdown. |
| `turn_generation_ms` | All attempts plus validation, up to the successful commit call. It excludes final SQLite commit latency. |
| `peak_rss_bytes` | OS high-water RSS. MLX uses RUSAGE_SELF; GGUF uses reaped child-process resource usage. It is not the same as total unified GPU allocation. |
| `mlx_peak_bytes` | MLX allocator's peak bytes. Never add this to RSS; they can overlap. |
| `swap_before_bytes`, `swap_after_bytes` | System-wide macOS swap usage snapshots, not memory attributable solely to this worker. Missing access is null. |
| `sampled_peak_rss_bytes` | GGUF best-effort RSS samples at 500 ms intervals. Requires OS process-inspection access. It can miss short spikes. |
| `status`, `attempt` | Separate first-pass validity, successful repairs, exhausted retries and runtime failures. |

Review Activity Monitor's Memory Pressure alongside these counters. Allocator limits and sampled RSS are useful guardrails, not a system-wide no-swap guarantee. Existing swap can remain allocated after worker exit; successful exit does not imply the system swap counter immediately returns to zero. File-cache warming can reduce subsequent load time even though each worker is fresh.

## Narrative scoring

Read the raw responses and before/after states in each JSON export. Score each dimension from 1 (poor) to 5 (strong), with a cited example from a turn:

| Dimension | What to inspect |
|---|---|
| Coherence | A plausible causal link between the action and its consequences; no unexplained jumps. |
| Dialogue | Mira's voice remains distinctive and responds to the actual question. |
| Character consistency | Canonical appearance, personality, promises and relationships remain consistent in prose as well as state. |
| Instruction following | Player agency is respected; choices are meaningful; facts in narration match state changes. |
| Memory | Turn four preserves important discoveries and unresolved commitments in its cumulative summary. |
| JSON reliability | First-pass acceptance rate, repair rate, and semantic errors. Grammar conformity alone is insufficient. |

The initial four turns are a smoke workload, not a statistically adequate quality ranking. A later comparison should add at least 20 turns per model, inventory edge cases, location changes, returning characters, and delayed plot payoffs. Preserve seed, quantization, runtime version and revision in each report. Identical seeds across different runtimes do not imply identical token sequences.

## Selection rule

Keep 12B if it produces materially better stories at acceptable measured turn latency and memory pressure. If its repeated runs show poor responsiveness or increased pressure/swap, run E4B on the same workload and review the quality/latency tradeoff. Avoid automatic mid-story model switching before recording a comparison; the user should know which model generated each turn.
