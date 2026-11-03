"""CPU-only safety/mapping tests for the disposable packed-tile component."""

from contextlib import redirect_stdout
from copy import deepcopy
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import numpy as np

import experiment_packed_tiles as assay


def fixture_document():
    rng = np.random.default_rng(839)
    n, edges = 32, 3 * 256 + 17
    # Canonical IDs interleave tiles; decoder remainder deliberately stays outside tiles.
    ids = np.arange(3 * 256).reshape(256, 3).T
    blocks = [dict(source_start=0, target_start=16, route=0, cadence=0, ids=ids[0].tolist()),
              dict(source_start=16, target_start=16, route=1, cadence=1, ids=ids[1].tolist()),
              dict(source_start=0, target_start=0, route=2, cadence=3, ids=ids[2].tolist())]
    document = dict(schema_version=1, neurons=n, synapse_count=edges,
                    recurrent_edge_count=3 * 256, recurrent_ids=list(range(3 * 256)),
                    weights=rng.uniform(-0.2, 0.2, edges).astype(np.float32).tolist(), blocks=blocks,
                    samples=[dict(activations=rng.uniform(-1, 1, n).astype(np.float32).tolist(),
                                  effective_weight_offsets=rng.uniform(-0.03, 0.03, edges).astype(np.float32).tolist(),
                                  enabled_routes=mask, microstep=step, expected_inputs=[0.0] * n)
                             for mask, step in [([True, True, True], 0), ([True, True, True], 1),
                                                ([False, True, True], 2), ([False, False, False], 0)]])
    # Independent implementation of native per-target, canonical global-ID reduction.
    by_id = {}
    for block in blocks:
        for slot, edge in enumerate(block["ids"]):
            by_id[edge] = (block, block["source_start"] + slot % 16, block["target_start"] + slot // 16)
    for sample in document["samples"]:
        expected = np.zeros(n, dtype=np.float32)
        for edge in sorted(by_id):
            block, source, target = by_id[edge]
            cadence, step = block["cadence"], sample["microstep"]
            fires = cadence == 0 or cadence in (1, 2) and step % 2 == 0 or cadence in (3, 4) and step == 0
            if sample["enabled_routes"][block["route"]] and fires:
                effective = np.float32(document["weights"][edge]) + np.float32(sample["effective_weight_offsets"][edge])
                expected[target] += np.float32(sample["activations"][source]) * effective
        sample["expected_inputs"] = expected.tolist()
    return document


class PackedTileTests(unittest.TestCase):
    def setUp(self):
        self.document = fixture_document()
        self.fixture = assay.parse_fixture(self.document)
        self.packed = assay.pack_fixture(self.fixture)

    def test_canonical_order_and_masks_match_independent_expected(self):
        actual = assay.canonical_inputs(self.fixture)
        self.assertEqual(actual.tobytes(), self.fixture.expected.tobytes())
        np.testing.assert_array_equal(self.fixture.masks, [[True, True, False, False],
                                                          [True, False, True, False],
                                                          [True, False, False, False]])
        for cadence in range(7):
            expected = [True] * 5 if cadence == 0 else ([True, False, True, False, True]
                       if cadence in (1, 2) else ([True, False, False, False, False]
                       if cadence in (3, 4) else [False] * 5))
            self.assertEqual([assay.route_fires(cadence, step) for step in range(5)], expected)

    def test_pack_roundtrip_preserves_all_bytes_including_decoder_and_integer_ages(self):
        arrays = [self.fixture.weights, self.fixture.offsets,
                  np.arange(self.fixture.weights.size, dtype=np.uint64) + 16_777_217]
        signed = self.fixture.weights.copy()
        signed[0], signed[-1] = np.float32(-0.0), np.float32(-0.0)
        arrays.append(signed)
        for canonical in arrays:
            original = canonical.tobytes()
            packed = assay.pack_edges(canonical, self.fixture.ids)
            restored = assay.unpack_edges(packed, self.fixture.ids, canonical)
            self.assertEqual(restored.dtype, canonical.dtype)
            self.assertEqual(restored.tobytes(), original)
            self.assertEqual(canonical.tobytes(), original)
        self.assertTrue(all(assay.verify_identities(self.fixture).values()))

    def test_independent_window_offsets_dense_drift_and_no_input_mutation(self):
        before = deepcopy(self.document)
        canonical_bytes = {key: value.tobytes() for key, value in vars(self.fixture).items()
                           if isinstance(value, np.ndarray)}
        output = assay.packed_inputs(self.fixture, self.packed)
        np.testing.assert_allclose(output, self.fixture.expected, atol=1e-6, rtol=1e-5)
        np.testing.assert_array_equal(output[-1], 0)
        for sample in range(len(self.fixture.activations)):
            single = deepcopy(self.document)
            single["samples"] = [single["samples"][sample]]
            parsed = assay.parse_fixture(single)
            one = assay.packed_inputs(parsed, assay.pack_fixture(parsed))
            np.testing.assert_allclose(one[0], output[sample], atol=1e-6, rtol=1e-5)
        self.assertEqual(self.document, before)
        for key, original in canonical_bytes.items():
            self.assertEqual(getattr(self.fixture, key).tobytes(), original)
        self.assertGreater(assay.drift(output, self.fixture.expected)["bit_different_values"], 0)

    def test_rejects_missing_duplicate_out_of_range_or_bad_slot_ids(self):
        for mutate in (lambda d: d["blocks"][0]["ids"].pop(),
                       lambda d: d["blocks"][0]["ids"].__setitem__(0, d["blocks"][1]["ids"][0]),
                       lambda d: d["blocks"][0]["ids"].__setitem__(0, d["synapse_count"]),
                       lambda d: d["recurrent_ids"].pop(),
                       lambda d: d["blocks"][0].__setitem__("source_start", 17)):
            invalid = deepcopy(self.document)
            mutate(invalid)
            with self.assertRaises(ValueError):
                assay.parse_fixture(invalid)

    def test_rejects_ambiguous_masks_nonfinite_or_shape_mismatch(self):
        for mutate in (lambda d: d["samples"][0]["enabled_routes"].__setitem__(0, 1),
                       lambda d: d["samples"][0]["enabled_routes"].pop(),
                       lambda d: d["weights"].__setitem__(0, float("nan")),
                       lambda d: d["samples"][0]["effective_weight_offsets"].pop(),
                       lambda d: d["samples"][0].__setitem__("microstep", -1)):
            invalid = deepcopy(self.document)
            mutate(invalid)
            with self.assertRaises(ValueError):
                assay.parse_fixture(invalid)

    def test_local_vjp_global_ids_masks_and_finite_difference(self):
        cotangent = np.cos(np.arange(self.fixture.activations.size, dtype=np.float32)).reshape(self.fixture.activations.shape)
        canonical = assay.canonical_vjp(self.fixture, cotangent)
        candidate = assay.packed_vjp(self.fixture, self.packed, cotangent)
        for actual, expected in zip(candidate, canonical):
            np.testing.assert_allclose(actual, expected, atol=1e-6, rtol=1e-5)
        # Perturb one shared genetic edge, one window-specific offset and one activation.
        # This is only the local linear reduction, not a recurrent neural gradient.
        epsilon = 1e-3
        cases = [("weights", 0, (0,), canonical[0][0]),
                 ("offsets", 0, (0, 0), canonical[1][0, 0]),
                 ("activations", 0, (0, 2), canonical[2][0, 2])]
        for name, _, index, expected in cases:
            losses = []
            for sign in (-1, 1):
                changed = deepcopy(self.document)
                if name == "weights":
                    changed["weights"][index[0]] += sign * epsilon
                else:
                    key = "effective_weight_offsets" if name == "offsets" else "activations"
                    changed["samples"][index[0]][key][index[1]] += sign * epsilon
                parsed = assay.parse_fixture(changed)
                losses.append(float(np.sum(assay.canonical_inputs(parsed).astype(np.float64) * cotangent)))
            self.assertAlmostEqual((losses[1] - losses[0]) / (2 * epsilon), float(expected), delta=4e-4)
        np.testing.assert_array_equal(candidate[0][-17:], 0)
        np.testing.assert_array_equal(candidate[1][-1], 0)

    def test_precision_context_restores_and_does_not_mix_apis(self):
        new = SimpleNamespace(fp32_precision="ieee")
        torch = SimpleNamespace(backends=SimpleNamespace(cuda=SimpleNamespace(matmul=new)))
        with assay.matmul_precision(torch, True) as api:
            self.assertEqual(api, "fp32_precision")
            self.assertEqual(new.fp32_precision, "tf32")
            with assay.matmul_precision(torch, False):
                self.assertEqual(new.fp32_precision, "ieee")
        self.assertEqual(new.fp32_precision, "ieee")
        self.assertFalse(hasattr(new, "allow_tf32"))
        old = SimpleNamespace(allow_tf32=True)
        torch.backends.cuda.matmul = old
        with self.assertRaisesRegex(RuntimeError, "deliberate"):
            with assay.matmul_precision(torch, False):
                self.assertFalse(old.allow_tf32)
                raise RuntimeError("deliberate")
        self.assertTrue(old.allow_tf32)

    def test_cpu_report_and_native_oracle_mismatch_rejected(self):
        report = assay.run(self.document, repeats=1, warmup=0)
        self.assertNotIn("cuda", report)
        self.assertEqual(report["canonical_vs_native"]["bit_different_values"], 0)
        self.assertGreater(report["cpu"]["vs_canonical"]["bit_different_values"], 0)
        self.assertTrue(all(d["within_tolerance"] for d in report["cpu_local_linear_vjp"].values()))
        invalid = deepcopy(self.document)
        invalid["samples"][0]["expected_inputs"][0] += 1
        with self.assertRaisesRegex(ValueError, "canonical FP32 oracle"):
            assay.run(invalid, repeats=1, warmup=0)

    def test_cuda_without_torch_refuses_without_cpu_fallback(self):
        with patch.dict("sys.modules", {"torch": None}):
            result = assay.cuda_assay(self.fixture, self.packed, self.fixture.expected, 1, 0, 1e-4, 1e-5)
        self.assertEqual(result["status"], "unavailable")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fresh-public.json"
            path.write_text(json.dumps(self.document))
            raw = path.read_bytes()
            output = io.StringIO()
            with patch.dict("sys.modules", {"torch": None}), redirect_stdout(output):
                code = assay.main([str(path), "--device", "cuda", "--repeats", "1", "--warmup", "0"])
            self.assertEqual(code, 2)
            self.assertEqual(json.loads(output.getvalue())["cuda"]["status"], "unavailable")
            self.assertEqual(path.read_bytes(), raw)

    def test_cuda_global_tf32_override_refuses_before_device_work(self):
        torch = SimpleNamespace(cuda=SimpleNamespace(is_available=lambda: True),
                                version=SimpleNamespace(cuda="test"))
        with patch.dict("sys.modules", {"torch": torch}), \
                patch.dict("os.environ", {"TORCH_ALLOW_TF32_CUBLAS_OVERRIDE": "1"}):
            result = assay.cuda_assay(self.fixture, self.packed, self.fixture.expected, 1, 0, 1e-4, 1e-5)
        self.assertEqual(result["status"], "unavailable")
        self.assertIn("FP32 offset", result["reason"])


if __name__ == "__main__":
    unittest.main()
