import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
import numpy as np

ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


image_model = load('image_model', 'workers/z_image/model.py')
image_worker = load('image_worker', 'workers/z_image/worker.py')
story_model = load('story_model', 'workers/gemma/model.py')
story_worker = load('story_worker', 'workers/gemma/worker.py')


class ModelReplacementTests(unittest.TestCase):
    def test_offloaded_models_are_refused_without_opening_them(self):
        for worker in (image_worker, story_worker):
            placeholder = SimpleNamespace(stat=lambda: SimpleNamespace(st_flags=0x40000000))
            with self.assertRaisesRegex(ValueError, 'offloaded to iCloud'):
                worker.require_downloaded(placeholder)
            worker.require_downloaded(SimpleNamespace(stat=lambda: SimpleNamespace(st_flags=0)))
            worker.require_downloaded(SimpleNamespace(stat=lambda: SimpleNamespace()))

    def test_q8_repacking_preserves_signed_extremes_and_scales(self):
        # Every signed-byte value, negative/subnormal/zero scales and multiple groups.
        values = np.arange(-128, 128, dtype=np.int16).astype(np.int8).reshape(2, 4, 32)
        scales = np.array([[0, .5, -.125, 1], [2**-24, 32, -8, .003]], dtype='<f2')
        blocks = np.empty((2, 4, 34), dtype=np.uint8)
        blocks[:, :, :2] = scales.view(np.uint8).reshape(2, 4, 2)
        blocks[:, :, 2:] = values.view(np.uint8)
        packed, d, bias = image_model.pack_q8_0(blocks, (2, 128))
        q = packed.view(np.uint8).reshape(2, 4, 32).astype(np.float32)
        actual = q*d[..., None] + bias[..., None]
        np.testing.assert_array_equal(actual, values.astype(np.float32)*scales.astype(np.float32)[..., None])

    def test_q8_repacking_rejects_invalid_scale(self):
        blocks = np.zeros((1,1,34), dtype=np.uint8)
        blocks[0,0,:2] = np.array([np.inf], dtype='<f2').view(np.uint8)
        with self.assertRaisesRegex(ValueError, 'Non-finite'):
            image_model.pack_q8_0(blocks, (1,32))

    def test_encoder_rejects_unknown_layers_and_output_head(self):
        for key in ['blk.36.attn_q.weight','output.weight','unexpected.weight']:
            with self.assertRaises(ValueError):
                image_model.converted_key(key)

    def test_image_validation_preserves_existing_output(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp)/'scene.png'
            target.write_bytes(b'keep me')
            request = dict(protocol_version=1,prompt='A scene',output_path=str(target),
                settings=dict(width=512,height=768,steps=9,seed=42,memory_limit_bytes=9*1024**3))
            with self.assertRaisesRegex(ValueError, 'new absolute PNG'):
                image_worker.validate_request(request)
            self.assertEqual(target.read_bytes(), b'keep me')
            request['settings']['steps']=20
            with self.assertRaisesRegex(ValueError, '4–12'):
                image_worker.validate_request(request)

    def test_story_weight_hash_detects_same_size_corruption(self):
        with tempfile.TemporaryDirectory() as temp:
            target=Path(temp)/'weights'
            target.write_bytes(b'abc')
            detail={'bytes':3,'sha256':'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'}
            story_model.verify_file(target,detail)
            target.write_bytes(b'abd')
            with self.assertRaisesRegex(ValueError,'checksum'):
                story_model.verify_file(target,detail)


if __name__=='__main__':
    unittest.main()
