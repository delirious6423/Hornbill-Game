"""Exercise the actual decoder grammar against previously observed failures."""
import copy
import json
from pathlib import Path
import re
import subprocess
import unittest

from outlines_core.json_schema import build_regex_from_schema

ROOT = Path(__file__).resolve().parents[2]


class DecoderContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Generate the contract from current Rust types, not a stale fixture.
        command = [str(ROOT / "scripts/cargo.sh"), "run", "--locked", "--quiet", "--", "schema"]
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True,
                                check=True, timeout=90)
        cls.schema = json.loads(result.stdout)
        cls.pattern = re.compile(build_regex_from_schema(json.dumps(cls.schema)))
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
        return self.pattern.fullmatch(encoded) is not None

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


if __name__ == "__main__":
    unittest.main()
