import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("gemma_worker",ROOT/"workers/gemma/worker.py")
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class WorkerProtocolTests(unittest.TestCase):
    def test_worker_validates_envelope_without_importing_mlx(self):
        for settings in ({},{"protocol_version":2}):
            with self.assertRaises((ValueError,KeyError)):
                module.validate_request(settings)

    def test_bad_request_returns_one_machine_readable_error(self):
        result=subprocess.run([sys.executable,str(ROOT/"workers/gemma/worker.py")],input="{broken",text=True,capture_output=True,check=True)
        envelope=json.loads(result.stdout)
        self.assertEqual(envelope["protocol_version"],1)
        self.assertEqual(envelope["raw_text"],"")
        self.assertIn("JSONDecodeError",envelope["error"])
        self.assertGreater(envelope["metadata"]["peak_rss_bytes"],0)

    def test_oversized_request_is_refused(self):
        result=subprocess.run([sys.executable,str(ROOT/"workers/gemma/worker.py")],input="x"*(256*1024+1),text=True,capture_output=True,check=True)
        self.assertIn("256 KiB",json.loads(result.stdout)["error"])

    def test_local_model_is_required_for_gguf(self):
        settings={"max_tokens":1024,"context_tokens":6144,"temperature":0.6,"seed":42,"memory_limit_bytes":10*1024**3}
        result=subprocess.run([sys.executable,str(ROOT/"workers/gguf/worker.py"),"--model","/nonexistent/model.gguf"],
                              input=json.dumps({"protocol_version":1,"settings":settings}),text=True,capture_output=True,check=True)
        envelope=json.loads(result.stdout)
        self.assertIn("existing local GGUF",envelope["error"])
        self.assertEqual(envelope["raw_text"],"")

    def test_image_worker_rejects_bad_protocol_without_importing_metal(self):
        result=subprocess.run([sys.executable,str(ROOT/"workers/z_image/worker.py")],input='{"protocol_version":2}',text=True,capture_output=True,check=True)
        envelope=json.loads(result.stdout)
        self.assertIn("Unsupported image protocol",envelope["error"])
        self.assertEqual(envelope["raw_text"],"")

    def test_image_worker_bounds_input_before_loading_weights(self):
        result=subprocess.run([sys.executable,str(ROOT/"workers/z_image/worker.py")],input="x"*(128*1024+1),text=True,capture_output=True,check=True)
        self.assertIn("128 KiB",json.loads(result.stdout)["error"])


if __name__=="__main__":
    unittest.main()
