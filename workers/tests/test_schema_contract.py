"""Exercise the actual decoder grammar against previously observed failures."""
import copy
import json
from pathlib import Path
import importlib.util
import subprocess
import unittest

from tokenizers import Tokenizer, models, pre_tokenizers, decoders
from transformers import PreTrainedTokenizerFast

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("structured", ROOT / "workers/gemma/structured.py")
structured = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(structured)


class DecoderContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Generate the contract from current Rust types, not a stale fixture.
        command = [str(ROOT / "scripts/cargo.sh"), "run", "--locked", "--quiet", "--", "schema"]
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True,
                                check=True, timeout=90)
        cls.schema = json.loads(result.stdout)
        vocab = {"<eos>": 0, "<unk>": 1}
        vocab.update({char: index + 2 for index, char in enumerate(sorted(pre_tokenizers.ByteLevel.alphabet()))})
        backend = Tokenizer(models.BPE(vocab=vocab, merges=[], unk_token="<unk>"))
        backend.pre_tokenizer = pre_tokenizers.ByteLevel(add_prefix_space=False, use_regex=False)
        backend.decoder = decoders.ByteLevel()
        cls.tokenizer = PreTrainedTokenizerFast(tokenizer_object=backend, eos_token="<eos>", unk_token="<unk>")
        cls.example = json.loads((ROOT / "prompts/demo_turn.json").read_text())

    def ordered(self, value, schema):
        if "$ref" in schema:
            schema = self.schema["$defs"][schema["$ref"].rsplit("/", 1)[-1]]
        if isinstance(value, dict):
            return {key: self.ordered(value[key], definition)
                    for key, definition in schema["properties"].items() if key in value}
        if isinstance(value, list):
            return [self.ordered(item, schema["items"]) for item in value]
        return value

    def accepted(self, value):
        encoded = json.dumps(self.ordered(value, self.schema), ensure_ascii=False)
        decoder = structured.StructuredDecoder(self.schema, self.tokenizer, {})
        tokens = self.tokenizer.encode(encoded, add_special_tokens=False)
        previous = None
        for token in tokens:
            mask = decoder.next_mask(previous)
            if not (int(mask[0, token // 32]) >> (token % 32)) & 1:
                return False
            previous = token
        decoder.next_mask(previous)
        return decoder.matcher.is_accepting() and not decoder.matcher.is_error()

    def test_decoder_closes_memory_array_before_runaway_repetition(self):
        value = copy.deepcopy(self.example)
        value["memory_updates"] = [value["memory_updates"][0]] * 5
        self.assertTrue(self.accepted(value), "five entries must remain representable")
        value["memory_updates"] *= 7
        self.assertFalse(self.accepted(value), "the observed 35-entry loop must be impossible")

    def test_proper_names_remain_prose_while_memory_tags_are_ids(self):
        value = copy.deepcopy(self.example)
        value["memory_updates"][0]["text"] = "Mira recognizes KESTREL-7 as Dr. Aris's frequency."
        value["memory_updates"][0]["tags"] = ["mira", "kestrel_7"]
        self.assertTrue(self.accepted(value))
        value["memory_updates"][0]["tags"] = ["KESTREL-7"]
        self.assertFalse(self.accepted(value))

    def test_inventory_grammar_rejects_zero_and_out_of_range_deltas(self):
        value = copy.deepcopy(self.example)
        value["state_changes"]["inventory"] = []
        self.assertTrue(self.accepted(value), "unchanged inventory must remain empty")
        for delta in (-100, -1, 1, 100):
            value["state_changes"]["inventory"] = [{"item": "flashlight", "delta": delta}]
            self.assertTrue(self.accepted(value), f"valid delta {delta}")
        for delta in (-101, 0, 101, 1.5):
            value["state_changes"]["inventory"] = [{"item": "flashlight", "delta": delta}]
            self.assertFalse(self.accepted(value), f"invalid delta {delta}")

    def test_decoder_refuses_invalid_sample_and_unknown_constraints(self):
        decoder = structured.StructuredDecoder(self.schema, self.tokenizer, {})
        invalid = self.tokenizer.encode("X", add_special_tokens=False)[0]
        with self.assertRaisesRegex(ValueError, "rejected sampled token"):
            decoder.next_mask(invalid)
        schema = {"type": "string", "format": "not-a-supported-format"}
        with self.assertRaises(ValueError):
            structured.StructuredDecoder(schema, self.tokenizer, {})

    def test_native_decoder_preserves_newlines_unicode_and_independent_turns(self):
        value = copy.deepcopy(self.example)
        value["narration"] = "Mira says: ‘A new clue!’\n雨 falls outside."
        self.assertTrue(self.accepted(value))
        self.assertTrue(self.accepted(self.example), "a fresh turn must start with an independent matcher")


if __name__ == "__main__":
    unittest.main()
