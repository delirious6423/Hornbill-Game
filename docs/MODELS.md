# Models and runtimes

All pins below are checked at install time (revision, size, hash). Sources of truth: `workers/gemma/models.json`, `workers/z_image/models.json`, `workers/z_image/model.py`, `workers/gguf/setup.py`, `workers/*/requirements.lock`, `scripts/setup.sh`, `rust-toolchain.toml`, `Cargo.toml`.

## Story models

| Profile (UI name) | Repository | Revision | Weights | Install dir | Runtime |
|---|---|---|---:|---|---|
| Gemma 12B (Q6) | [shoemoney/Gemma-4-12B-Abliterated-MLX-q6](https://huggingface.co/shoemoney/Gemma-4-12B-Abliterated-MLX-q6) | `edd4c19a3edc7d4637c729fcb776e190ba0b7b25` | 9,728,621,337 B | `models/12b-abliterated-q6` | MLX |
| Gemma E4B (8-bit) | [mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit](https://huggingface.co/mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit) | `db71b2644248b4f9e7e31ede5e6f631b870d8af5` | 7,988,669,526 B | `models/e4b-obliterated-8bit` | MLX |
| GGUF fallback (Gemma 12B Q4_K_M) | [unsloth/gemma-4-12b-it-GGUF](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/tree/fc034cfff751157913579611efad8462ac1be606) | `fc034cfff751157913579611efad8462ac1be606` | 7,121,861,440 B | `models/gguf-12b` | llama.cpp b11247 |

Directories are under the runtime folder. Gemma 12B (Q6) is the default. The GGUF fallback is the original instruction-tuned model, not an equivalent of the two MLX profiles.

- Install and select: `./scripts/model.sh download 12b|e4b`, `./scripts/model.sh use 12b|e4b` (sets the CLI default; the UI model choice is per story).
- GGUF: `./scripts/gguf.sh setup` downloads the llama.cpp b11247 macOS arm64 archive (SHA256 `ebf1ccab751a0a972dfb06225bc716174dffb0cf780206c03802e620b21f96e0`) and the GGUF (SHA256 `0a270ec9fe6b34f4a0d33992b6135117b484ebc4766ab76b51d4ae8c457e4c42`). `gguf.sh path` prints the model file.
- The GGUF worker sets DRY repetition controls (multiplier 0.8, base 1.75, allowed length 3, 512-token lookback) to stop repeated-word loops; they are stored in the audit.
- `mlx-lm` is pinned to a source revision because the PyPI release lacks the `gemma4_unified` mapping. Remote code is not enabled, and weights load with `strict=True`.
- Defaults: context 6,144 tokens, up to 1,536 output tokens, 256-token prefill chunks, 10 GiB MLX memory limit, 256 MiB cache limit.

## Image model

Z-Image-Turbo is assembled from two pinned parts. Both declare Apache 2.0.

| Part | Repository | Revision | Used |
|---|---|---|---|
| Renderer | [mflux-community/z-image-turbo-mflux-q4](https://huggingface.co/mflux-community/z-image-turbo-mflux-q4/tree/f427e257d8e6ffa03edd4d9ac554a05809da456c) | `f427e257d8e6ffa03edd4d9ac554a05809da456c` | Q4 transformer and VAE only |
| Text encoder | [BennyDaBall/Qwen3-4b-Z-Image-Turbo-AbliteratedV1](https://huggingface.co/BennyDaBall/Qwen3-4b-Z-Image-Turbo-AbliteratedV1/tree/ce497d288a7ddfd5d0f337c7139349d5d0236bfa) | `ce497d288a7ddfd5d0f337c7139349d5d0236bfa` | `Z-Image-AbliteratedV1.Q8_0.gguf` |

Encoder file: 4,280,405,248 bytes, SHA256 `6272f0f8db9e91f5ed748c51da27f646ae2d2cb2a28460a38294192288f85298`.

`./scripts/image.sh download` verifies each file, then repacks the GGUF Q8_0 blocks into MLX affine 8-bit (group size 32) without requantizing, checking the tensor shapes and sampled rows against both dequantizers. The converted encoder (about 5.0 GB) is written to `pipeline/text_encoder`; the GGUF stays in `encoder-source`. The install is `models/z-image-turbo-benny` with a `hornbill-model.json` receipt; a different existing install is never overwritten.

Defaults: 512×768, 9 steps (allowed 4–12), seed 42, 9 GiB memory limit (allowed 4–10 GiB), prompt limit 512 tokens. With a reference image, strength 0.6 runs the last four of the nine steps.

The upstream Z-Image license (commit `26f23eda626ffadda020b04ff79488e1d72004cd`, SHA256 `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`) is bundled as `workers/z_image/APACHE-2.0.txt`. Keep model cards and attribution with the weights. These are upstream declarations, not legal advice.

## Runtime versions

| Component | Version |
|---|---|
| Rust | 1.98.1 (`rust-toolchain.toml`; edition 2024, `rust-version = 1.98`) |
| Cargo dependencies | `Cargo.lock` (axum 0.8.9, rusqlite 0.40.2 bundled, tokio 1.53.1, clap 4.6.7) |
| Python | 3.12 via uv 0.12.20 |
| MLX | 0.32.3 (story and image environments) |
| mlx-lm | source `3051e26bc72b8ab426af17d14c5599447eb306a5` |
| llguidance | 1.8.0 |
| MFLUX | source `2924d0c7cd7104a1ab2f18f40d3bedcf47ba8b9c` (separate image environment) |
| gguf (Python) | 0.19.0 |
| llama.cpp | b11247 |

Python dependencies are locked in `workers/gemma/requirements.lock` and `workers/z_image/requirements.lock`; edit the `.in` files and recompile to change them.
