"""Small per-turn JSON matcher; no model or decoder state survives the worker."""
import importlib.metadata
import time

from llguidance import LLMatcher, LLTokenizer
from llguidance.hf import from_tokenizer
from llguidance.numpy import allocate_token_bitmask, fill_next_token_bitmask
from transformers import PreTrainedTokenizerFast


def create_matcher(schema, tokenizer):
    grammar = LLMatcher.grammar_from_json_schema(
        schema, overrides={"whitespace_pattern": r"[ ]?"})
    matcher = LLMatcher(tokenizer, grammar, log_level=0)
    if matcher.is_error():
        raise ValueError(f"Invalid story grammar: {matcher.get_error()}")
    warnings = matcher.get_grammar_warnings()
    if warnings:
        raise ValueError(f"Story grammar has unsupported constraints: {warnings}")
    return matcher


class StructuredDecoder:
    def __init__(self, schema, tokenizer, metadata):
        started = time.perf_counter()
        inner = tokenizer if isinstance(tokenizer, PreTrainedTokenizerFast) else tokenizer._tokenizer
        eos = getattr(tokenizer, "eos_token_ids", {tokenizer.eos_token_id})
        if isinstance(eos, int):
            eos = [eos]
        if not eos or any(value is None for value in eos):
            raise ValueError("Story tokenizer requires an EOS token")
        self.tokenizer = from_tokenizer(
            inner, eos_token=sorted(eos), slices=LLTokenizer.json_slices())
        metadata["decoder_tokenizer_ms"] = (time.perf_counter() - started) * 1000
        grammar_start = time.perf_counter()
        self.matcher = create_matcher(schema, self.tokenizer)
        metadata["schema_compile_ms"] = (time.perf_counter() - grammar_start) * 1000
        self.mask = allocate_token_bitmask(1, self.tokenizer.vocab_size)
        self.first = True
        self.metadata = metadata
        metadata.update(structured_decoder=f"llguidance/{importlib.metadata.version('llguidance')}",
                        schema_prepare_ms=(time.perf_counter() - started) * 1000,
                        decoder_mask_ms=0.0, decoder_mask_calls=0)

    def next_mask(self, last_token=None):
        started = time.perf_counter()
        if last_token is not None and not self.matcher.consume_token(last_token):
            raise ValueError(f"Story decoder rejected sampled token: {self.matcher.get_error()}")
        fill_next_token_bitmask(self.matcher, self.mask)
        if self.matcher.is_error():
            raise ValueError(f"Story decoder stopped: {self.matcher.get_error()}")
        if not self.mask.any():
            raise ValueError("Story decoder produced an empty token mask")
        self.metadata["decoder_mask_ms"] += (time.perf_counter() - started) * 1000
        self.metadata["decoder_mask_calls"] += 1
        return self.mask

    def __call__(self, token_ids, logits):
        import mlx.core as mx
        from llguidance.mlx import apply_token_bitmask

        if token_ids.ndim != 1 or logits.ndim != 2 or logits.shape[0] != 1:
            raise ValueError("Story decoder requires one unbatched sequence")
        # MLX evaluates lazily. Finish the preceding sampling operation before
        # advancing the matcher or replacing the mask used by that operation.
        mx.eval(token_ids)
        last_token = None if self.first else int(token_ids[-1].item())
        self.first = False
        return apply_token_bitmask(logits, self.next_mask(last_token))
