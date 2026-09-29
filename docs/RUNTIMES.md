# Runtime verification · 29 September 2026

The following are upstream availability/implementation checks. They do not substitute for hardware benchmarks. Repository revisions and model artifacts are pinned in the supplied manifests and dependency locks.

## Story runtimes

| Candidate | Current evidence | MVP decision |
|---|---|---|
| Python MLX / mlx-lm | Upstream commit `3051e26bc72b8ab426af17d14c5599447eb306a5` maps `gemma4_unified` to the Gemma 4 text implementation and discards unused multimodal tensors. Released 0.31.3 lacks that 12B mapping. | Primary Apple implementation, pinned source; four real Metal turns verified for both profiles. |
| llama.cpp / GGUF | Current native Gemma 4 parser and 12B GGUF artifacts exist. b11247 provides native templates, local token counting, grammar/schema output, and Metal/CPU builds. | Implemented portable fallback with an explicit CPU test mode. |
| mistral.rs | Maintained Rust engine with Gemma 4 multimodal support, Metal, quantization, and structured generation. Published front-page speed comparisons use NVIDIA hardware. | Credible later alternative; do not transfer CUDA numbers to a base M4. Prefer prequantized weights over quantizing full precision in 16 GB. |
| mlx-rs | Actively developed Rust bindings to the MLX array framework. The repository's examples are not a verified Gemma 4 12B application/runtime. | Do not implement our own transformer/tokenizer/quantization integration to make the first milestone all-Rust. |
| mlx-vlm | The community model cards name this multimodal path. Our input is text; loading image/audio machinery adds integration scope. | Keep available for future multimodal requirements; use the text-only mapping for Phase 1. |

Sources: [MLX-LM pinned mapping](https://github.com/ml-explore/mlx-lm/blob/3051e26bc72b8ab426af17d14c5599447eb306a5/mlx_lm/utils.py), [Gemma weight sanitization](https://github.com/ml-explore/mlx-lm/blob/3051e26bc72b8ab426af17d14c5599447eb306a5/mlx_lm/models/gemma4.py), [llama.cpp Gemma 4 parser](https://github.com/ggml-org/llama.cpp/blob/0bc845d35/common/parsers/gemma4.cpp), [llama.cpp server](https://github.com/ggml-org/llama.cpp/tree/b11247/tools/server), [mistral.rs](https://github.com/EricLBuehler/mistral.rs), [mlx-rs](https://github.com/oxiglade/mlx-rs).

## Exact story artifacts

| Profile | Repository | Revision | Weight bytes |
|---|---|---|---:|
| MLX 12B, 4-bit | `mlx-community/gemma-4-12B-it-4bit` | `73bcf09092aa277861d5a191b989b666f7f32e8f` | 6,741,039,511 |
| MLX E4B, 4-bit | `mlx-community/gemma-4-e4b-it-4bit` | `475b9088d29754a3379866cf5aeb6b41acd313c2` | 5,146,800,534 |
| GGUF 12B, Q4_K_M | `unsloth/gemma-4-12b-it-GGUF` | `fc034cfff751157913579611efad8462ac1be606` | 7,121,861,440 |

The E4B file is about 5.15 GB: its name is not a promise of a 2 GB download or resident footprint. The 12B MLX weights are about 6.74 GB. These are disk sizes, not runtime RAM. KV cache, tokenizer, activations, allocator cache, and the operating system all need additional memory. [MLX 12B model](https://huggingface.co/mlx-community/gemma-4-12B-it-4bit), [MLX E4B model](https://huggingface.co/mlx-community/gemma-4-e4b-it-4bit), [GGUF files](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/tree/fc034cfff751157913579611efad8462ac1be606).

Initial settings are a 6,144-token total context, up to 1,536 generated tokens, 256-token MLX prefill chunks, a 10 GiB MLX allocator budget, and a 256 MiB allocator cache. These are starting limits, not measured safe operating points for every application workload. KV quantization and speculative decoding are deferred until the baseline is measured.

## Qualified Qwen Image 2.1 runtime

The local worker now uses [MFLUX source 2924d0c](https://github.com/mflux-community/mflux/tree/2924d0c7cd7104a1ab2f18f40d3bedcf47ba8b9c/src/mflux/models/qwen21), with the newer LoRA mapping and text-prefix cache absent from the published 0.20.0 wheel. It is pinned separately from the story environment in workers/qwen_image/requirements.lock.

[Full-Q4 MFLUX pack](https://huggingface.co/mlx-community/Qwen-Image-2.1-mflux-q4/tree/746a58556820933a2df5c75887a2570f1ad200c0) provides both encoder and transformer Q4 weights. The worker narrowly enables quantized encoder construction for this verified pack. Prompt materialization before eviction follows the approach documented by [Rapid-MLX](https://github.com/raullenchai/Rapid-MLX/blob/778781e5e5715c500ab3b087d0bba48988914063/rapid_mlx/image/engine.py).

The [Viggle v0.2.1 rank-128 adapter](https://huggingface.co/Viggle/Qwen-Image-2.1-viggle-turbo/tree/bb26a0f38e5fe6c124aaccc9187a87eed5d9ed13) runs unmerged with six fixed sigmas: 1, .9375, .875, .75, .5, .25, then 0. All 454 keys matched 227 layers in the actual run. Do not substitute a generic low-step schedule or merge into BF16 weights.

The first 512×768 M4 image took 103.23 seconds and peaked at 5.13 GiB of MLX memory. Encoder/transformer eviction and worker exit were measured; tiled VAE decode is enabled and allocator cache is zero. See VALIDATION.md and measurements/qwen-first-image.json for the actual limits. Downloaded files total about 9.57 GiB; disk size is not resident memory.

The [official Qwen model](https://huggingface.co/Qwen/Qwen-Image-2.1) uses qwen-research terms. Preserve the downloaded base/LICENSE; inconsistent community Apache metadata does not replace the upstream license.

The browser reference workflow and a real img2img run are now verified. At 512×768 and reference strength 0.6, it used the last three updates of the six-step schedule: 43.34 seconds total, peak MLX 5.43 GiB and peak RSS 4.94 GiB, with the system swap counter unchanged. This is not directly comparable with a full six-update text-to-image run. See measurements/qwen-reference-image.json.

This low-memory path offers one composition reference via img2img. MFLUX native Qwen21 Edit uses a separate multimodal loader and is not yet qualified here. Do not claim multi-reference identity conditioning from an img2img result. GGUF image support exists upstream in stable-diffusion.cpp but is not an implemented Hornbill image backend; story GGUF is implemented.
