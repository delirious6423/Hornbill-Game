# Runtime and model selections

The user selected these replacements after checkpoint 0005. Old Qwen and original Gemma measurements remain historical evidence; they do not qualify the replacement weights. Installation pins exact revisions and verifies file sizes and hashes. Inference is offline.

## Story profiles

| Profile | Selected repository | Revision | Weight bytes |
|---|---|---|---:|
| 12B, MLX Q6 | [shoemoney/Gemma-4-12B-Abliterated-MLX-q6](https://huggingface.co/shoemoney/Gemma-4-12B-Abliterated-MLX-q6) | `edd4c19a3edc7d4637c729fcb776e190ba0b7b25` | 9,728,621,337 |
| E4B, MLX 8-bit | [mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit](https://huggingface.co/mlx-community/gemma-4-E4B-it-OBLITERATED-mlx-8Bit) | `db71b2644248b4f9e7e31ede5e6f631b870d8af5` | 7,988,669,526 |
| Story compatibility fallback, GGUF Q4_K_M | [unsloth/gemma-4-12b-it-GGUF](https://huggingface.co/unsloth/gemma-4-12b-it-GGUF/tree/fc034cfff751157913579611efad8462ac1be606) | `fc034cfff751157913579611efad8462ac1be606` | 7,121,861,440 |

The replacement MLX model cards declare Apache 2.0. Their downloaded cards retain attribution to their respective source models. The earlier lemuralabs 12B 8-bit suggestion was superseded and is not installed or selected. The GGUF fallback is the original instruction model, not an abliterated equivalent of either new MLX profile.

The local directories are `models/12b-abliterated-q6` and `models/e4b-obliterated-8bit`. `workers/gemma/models.json` is the shared selection authority for the installer, UI and benchmark runner. A prior profile receipt or incomplete installation is not reported as ready. `scripts/model.sh use 12b|e4b` verifies the pinned installation before switching the default CLI model; UI preferences remain per save.

MLX 0.32.3 and [mlx-lm source 3051e26](https://github.com/ml-explore/mlx-lm/tree/3051e26bc72b8ab426af17d14c5599447eb306a5) remain pinned. This revision already maps `gemma4_unified` to the Gemma text implementation and strips unused multimodal tensors in its official sanitizer. Hornbill keeps `strict=True` for the remaining model weights and does not enable remote code. This is why the 12B model card's mlx-vlm example does not require a second multimodal runtime for Hornbill's text-only input. Outlines 1.3.3 constrains the Rust-generated schema, and Rust validates state changes independently.

Default context is 6,144 tokens with up to 1,536 output tokens, 256-token prefill chunks, a 10 GiB MLX allocator limit and 256 MiB cache. File size is not resident memory. See [validation](VALIDATION.md) for observed allocations, swap and quality limits. The two workers never remain loaded together or overlap an image worker.

The selected 12B initially returned zero inventory deltas for unchanged items. Rust rejected both attempts without changing the save. Inventory instructions now explicitly use an empty array for unchanged counts, and the decoder allows only nonzero integers from -100 to 100. The schema uses an explicit numeric enum because the pinned Outlines Core ignores numeric `minimum`/`maximum`; a regression exercises the actual compiled grammar at both bounds and rejects zero, fractions and out-of-range values. Rust still validates ownership and inventory underflow independently.

The existing llama.cpp b11247 GGUF fallback retains private temporary-server supervision, schema output and CPU/Metal modes. It is the portability path for story generation. Linux/NVIDIA and Windows remain unqualified on this Mac.

## Z-Image-Turbo with the exact selected encoder

The image stack consists of two separately pinned components:

- Renderer: [mflux-community/z-image-turbo-mflux-q4](https://huggingface.co/mflux-community/z-image-turbo-mflux-q4/tree/f427e257d8e6ffa03edd4d9ac554a05809da456c), revision `f427e257d8e6ffa03edd4d9ac554a05809da456c`. Only its Q4 transformer and VAE are installed; its default text encoder is not used.
- Encoder: [BennyDaBall/Qwen3-4b-Z-Image-Turbo-AbliteratedV1](https://huggingface.co/BennyDaBall/Qwen3-4b-Z-Image-Turbo-AbliteratedV1/tree/ce497d288a7ddfd5d0f337c7139349d5d0236bfa), revision `ce497d288a7ddfd5d0f337c7139349d5d0236bfa`, exact file `Z-Image-AbliteratedV1.Q8_0.gguf`. Size: 4,280,405,248 bytes. SHA256: `6272f0f8db9e91f5ed748c51da27f646ae2d2cb2a28460a38294192288f85298`.

The user-linked GGUF is a Qwen3-4B text encoder for Z-Image, not a stand-alone image renderer. Hornbill uses that exact file. Its signed Q8_0 blocks are repacked into equivalent MLX affine Q8 blocks with group size 32, F32 scales and F32 biases. This is a storage adaptation without requantization. Every integer block is checked during repacking; actual MLX and GGUF dequantizers are compared at the first and last rows of all 253 matrices, with zero sampled error. The 398 source tensors map to the complete expected encoder. The native cache is 5,028,771,857 bytes, in addition to the retained GGUF. Receipt and generated-shard hashes live in `hornbill-model.json`; `workers/z_image/model.py` can re-verify them.

Both component cards declare Apache 2.0. The [upstream Z-Image license at commit 26f23ed](https://github.com/Tongyi-MAI/Z-Image/blob/26f23eda626ffadda020b04ff79488e1d72004cd/LICENSE) is bundled as `workers/z_image/APACHE-2.0.txt`; its SHA256 is `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`. Preserve the cards and attribution. These are recorded upstream declarations, not an independent legal opinion. Qwen Image 2.1 and its `qwen-research` license are historical and are no longer part of the active image backend.

MFLUX source `2924d0c7cd7104a1ab2f18f40d3bedcf47ba8b9c`, MLX 0.32.3 and gguf 0.19.0 are pinned in the separate image environment. The worker loads the encoder, materializes penultimate-layer prompt features and releases it; loads the Q4 transformer and denoises; releases the transformer and compiled closure; then loads the VAE for tiled decoding. For a reference, it briefly loads the VAE before denoising too. Process exit is the final reclamation boundary.

The selected tokenizer defaults to left padding, while MFLUX's encoder assumes valid tokens start at zero. The first integration run produced non-finite encoder features and a black PNG; that failed artifact is excluded from deliverables. The worker now explicitly right-pads, checks for non-finite features/latents/decoded pixels and rejects a bad result before writing. It checks the complete chat-templated prompt against 512 tokens rather than silently truncating it.

Default images use 512×768, nine denoising steps, seed 42 and a 9 GiB allocator ceiling. Supported step values are 4–12. Existing saved Qwen settings migrate once to nine steps while retaining the selected story model, scheduling, dimensions, seed and reference strength. Later explicit Z-Image step choices are preserved.

A reference uses img2img composition guidance. Strength 0.6 with nine steps runs the last four denoising updates; it is not directly comparable with nine-step text-to-image. A composition result does not prove face identity locking or multiple-reference conditioning. The current image runtime requires Apple Silicon/MLX; selecting a GGUF encoder does not by itself make the entire image pipeline portable.

## Storage and preservation

The user approved removing only superseded Qwen 2.1 and original Gemma 12B/E4B downloads after each replacement passes checks. Saves, generated images, source, test evidence and the GGUF fallback remain local and preserved. Fresh installation needs roughly 45–50 GB for the selected image assets, optional story profiles, runtimes and build files; conversion also needs temporary free space. Every installer checks space before downloading, and the image converter shares the inference lease.
