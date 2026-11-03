#!/usr/bin/env python3
"""Disposable recurrent-sum component assay, never a training backend.

Usage: python scripts/experiment_packed_tiles.py fresh-fixture.json --device cpu
       python scripts/experiment_packed_tiles.py fresh-fixture.json --device cuda

Only accept a fresh public fixture exported by the Rust experiment. Each column
is an independent replay window, never the next tick of another column. Global
IDs, FP32 master weights and uncovered (e.g. decoder) entries remain canonical.
This tests a linear synapse reduction and its local VJP; it does not implement
neural recurrence, native strict clamp derivatives, PPO, Adam or checkpoints.

Dense splitting changes rounding: canonical a*(w+offset), global-ID order,
becomes tile GEMM(w,a) + per-window matvec(offset,a), then tile-order scatter.
Report that drift even with TF32 disabled; bit equality is not promised.

CUDA is optional and never emulated. The only TF32-eligible operation is the
genetic GEMM. Offset matvecs always request IEEE FP32. An enabled flag proves
permission, not Tensor Core execution; profiler kernel/counter evidence is still
required. API/timing reference: https://docs.pytorch.org/docs/stable/notes/cuda.html
"""

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import platform
import sys
import time

import numpy as np


TILE = 16


def integer(value, name, minimum=0):
    if type(value) is not int or value < minimum:
        raise ValueError(f"{name} must be an integer >= {minimum}")
    return value


def f32(value, shape, name):
    array = np.asarray(value, dtype=np.float32)
    if array.shape != shape or not np.isfinite(array).all():
        raise ValueError(f"{name} must have shape {shape} and finite FP32 values")
    return array


def route_fires(cadence, microstep):
    """Native raw enum, not a literal period; local step resets each tick."""
    return cadence == 0 or (cadence in (1, 2) and microstep % 2 == 0) or (
        cadence in (3, 4) and microstep == 0
    )


@dataclass
class Fixture:
    neurons: int
    weights: np.ndarray
    recurrent_ids: np.ndarray
    ids: np.ndarray                 # [tile, target slot, source slot]
    sources: np.ndarray             # [tile, source slot]
    targets: np.ndarray             # [tile, target slot]
    routes: np.ndarray
    cadences: np.ndarray
    activations: np.ndarray         # [independent window, neuron]
    offsets: np.ndarray             # [independent window, global ID]
    masks: np.ndarray               # [tile, independent window]
    expected: np.ndarray


def parse_fixture(document):
    if document.get("schema_version") != 1:
        raise ValueError("schema_version must be 1")
    n = integer(document["neurons"], "neurons", TILE)
    edges = integer(document["synapse_count"], "synapse_count", 1)
    weights = f32(document["weights"], (edges,), "weights")
    recurrent = document["recurrent_ids"]
    for edge in recurrent:
        integer(edge, "recurrent ID")
    if len(set(recurrent)) != len(recurrent) or any(edge >= edges for edge in recurrent):
        raise ValueError("recurrent IDs must be unique canonical IDs within weights")
    count = integer(document["recurrent_edge_count"], "recurrent_edge_count", 1)
    if len(recurrent) != count:
        raise ValueError("recurrent_edge_count differs from recurrent_ids")
    ids, sources, targets, routes, cadences = [], [], [], [], []
    for block in document["blocks"]:
        source = integer(block["source_start"], "source_start")
        target = integer(block["target_start"], "target_start")
        if source + TILE > n or target + TILE > n:
            raise ValueError("tile endpoint exceeds neurons")
        raw_ids = block["ids"]
        if len(raw_ids) != TILE * TILE:
            raise ValueError("each tile needs 256 target-major/source-minor IDs")
        for edge in raw_ids:
            integer(edge, "block ID")
        ids.append(raw_ids)
        sources.append(np.arange(source, source + TILE))
        targets.append(np.arange(target, target + TILE))
        routes.append(integer(block["route"], "route"))
        cadences.append(integer(block["cadence"], "cadence"))
    flattened = [edge for block in ids for edge in block]
    if len(set(flattened)) != len(flattened) or set(flattened) != set(recurrent):
        raise ValueError("block IDs must cover recurrent_ids exactly once")
    if not ids:
        raise ValueError("at least one recurrent tile is required")
    activations, offsets, masks, expected = [], [], [], []
    for sample in document["samples"]:
        flags = sample["enabled_routes"]
        if any(type(flag) is not bool for flag in flags) or len(flags) <= max(routes):
            raise ValueError("enabled_routes must contain bools for every block route")
        step = integer(sample["microstep"], "microstep")
        activations.append(f32(sample["activations"], (n,), "activations"))
        offsets.append(f32(sample["effective_weight_offsets"], (edges,), "offsets"))
        expected.append(f32(sample["expected_inputs"], (n,), "expected_inputs"))
        masks.append([flags[r] and route_fires(c, step) for r, c in zip(routes, cadences)])
    if not activations:
        raise ValueError("at least one independent sample is required")
    fixture = Fixture(n, weights, np.asarray(sorted(recurrent), dtype=np.int64),
                      np.asarray(ids, dtype=np.int64).reshape(-1, TILE, TILE),
                      np.asarray(sources), np.asarray(targets), np.asarray(routes),
                      np.asarray(cadences), np.stack(activations), np.stack(offsets),
                      np.asarray(masks, dtype=bool).T.copy(), np.stack(expected))
    # No resident/private state is loaded; make the supplied canonical inputs immutable.
    for array in vars(fixture).values():
        if isinstance(array, np.ndarray):
            array.flags.writeable = False
    return fixture


def pack_edges(canonical, ids):
    """Works for FP32 moments or integer ages too, without changing their dtype."""
    return canonical[..., ids].copy()


def unpack_edges(packed, ids, canonical_base):
    if packed.shape != canonical_base.shape[:-1] + ids.shape or packed.dtype != canonical_base.dtype:
        raise ValueError("packed shape/dtype must match canonical base")
    result = canonical_base.copy()
    result[..., ids] = packed
    return result


def edge_mapping(fixture):
    source = np.zeros(fixture.weights.size, dtype=np.int64)
    target = source.copy()
    tile = source.copy()
    for block, ids in enumerate(fixture.ids):
        source[ids] = fixture.sources[block][None, :]
        target[ids] = fixture.targets[block][:, None]
        tile[ids] = block
    return source, target, tile


def canonical_inputs(fixture):
    """FP32 add/multiply, global-ID ascending within every target, no FMA."""
    source, target, tile = edge_mapping(fixture)
    result = np.zeros_like(fixture.activations)
    for edge in fixture.recurrent_ids:
        active = fixture.masks[tile[edge]]
        effective = fixture.weights[edge] + fixture.offsets[active, edge]
        result[active, target[edge]] += fixture.activations[active, source[edge]] * effective
    return result


@dataclass
class Packed:
    weights: np.ndarray             # [tile, 16, 16], shared across windows
    activations: np.ndarray         # [tile, 16, B]
    offsets: np.ndarray             # [tile, B, 16, 16], distinct per window
    masks: np.ndarray               # [tile, B]


def pack_fixture(fixture):
    return Packed(pack_edges(fixture.weights, fixture.ids),
                  fixture.activations[:, fixture.sources].transpose(1, 2, 0).copy(),
                  pack_edges(fixture.offsets, fixture.ids).transpose(1, 0, 2, 3).copy(),
                  fixture.masks.copy())


def packed_inputs(fixture, packed):
    genetic = packed.weights @ packed.activations
    offset = (packed.offsets @ packed.activations.transpose(0, 2, 1)[..., None])[..., 0]
    contributions = genetic + offset.transpose(0, 2, 1)
    result = np.zeros_like(fixture.activations)
    for tile, targets in enumerate(fixture.targets):
        result[:, targets] += np.where(packed.masks[tile, :, None], contributions[tile].T, np.float32(0))
    return result


def canonical_vjp(fixture, cotangent):
    """Local vector-Jacobian product of the linear sum only, no neural autograd."""
    source, target, tile = edge_mapping(fixture)
    dw = np.zeros_like(fixture.weights)
    do = np.zeros_like(fixture.offsets)
    da = np.zeros_like(fixture.activations)
    for edge in fixture.recurrent_ids:
        active = fixture.masks[tile[edge]]
        upstream = cotangent[active, target[edge]]
        do[active, edge] = upstream * fixture.activations[active, source[edge]]
        dw[edge] = do[:, edge].sum(dtype=np.float32)
        da[active, source[edge]] += upstream * (fixture.weights[edge] + fixture.offsets[active, edge])
    return dw, do, da


def packed_vjp(fixture, packed, cotangent):
    upstream = cotangent[:, fixture.targets].transpose(1, 2, 0).copy()
    upstream *= packed.masks[:, None, :]
    dw = unpack_edges(upstream @ packed.activations.transpose(0, 2, 1), fixture.ids,
                      np.zeros_like(fixture.weights))
    do = (upstream.transpose(0, 2, 1)[..., :, None]
          * packed.activations.transpose(0, 2, 1)[..., None, :])
    do = unpack_edges(do.transpose(1, 0, 2, 3), fixture.ids, np.zeros_like(fixture.offsets))
    genetic = packed.weights.transpose(0, 2, 1) @ upstream
    offsets = (packed.offsets.transpose(0, 1, 3, 2)
               @ upstream.transpose(0, 2, 1)[..., None])[..., 0]
    da = np.zeros_like(fixture.activations)
    for tile, sources in enumerate(fixture.sources):
        da[:, sources] += genetic[tile].T + offsets[tile]
    return dw, do, da


def drift(actual, expected, rtol=1e-4, atol=1e-5):
    delta = actual.astype(np.float64) - expected.astype(np.float64)
    finite = bool(np.isfinite(actual).all())
    return {"bit_different_values": int(np.count_nonzero(actual.view(np.uint32) != expected.view(np.uint32))),
            "max_abs": float(np.abs(delta).max()),
            "relative_l2": float(np.linalg.norm(delta) / max(np.linalg.norm(expected.astype(np.float64)), 1e-30)),
            "within_tolerance": finite and bool(np.allclose(actual, expected, rtol=rtol, atol=atol)),
            "rtol": rtol, "atol": atol}


def verify_identities(fixture):
    # Synthetic optimizer-like arrays test only layout preservation, not an Adam step.
    ages = np.arange(fixture.weights.size, dtype=np.uint64) + 16_777_217
    moments = np.linspace(-0.25, 0.25, fixture.weights.size, dtype=np.float32)
    arrays = {"weights": fixture.weights, "sample_offsets": fixture.offsets,
              "synthetic_fp32_moments": moments, "synthetic_integer_ages": ages}
    for name, canonical in arrays.items():
        before = canonical.tobytes()
        unpacked = unpack_edges(pack_edges(canonical, fixture.ids), fixture.ids, canonical)
        if before != canonical.tobytes() or before != unpacked.tobytes():
            raise ValueError(f"packing changed canonical bytes: {name}")
    return {name: True for name in arrays}


@contextmanager
def matmul_precision(torch, tf32):
    """Never mix PyTorch's old/new precision APIs; restore caller's settings."""
    backend = torch.backends.cuda.matmul
    if hasattr(backend, "fp32_precision"):
        previous = backend.fp32_precision
        backend.fp32_precision = "tf32" if tf32 else "ieee"
        try:
            if backend.fp32_precision != ("tf32" if tf32 else "ieee"):
                raise RuntimeError("CUDA matmul precision request was not applied")
            yield "fp32_precision"
        finally:
            backend.fp32_precision = previous
    else:
        previous = backend.allow_tf32
        backend.allow_tf32 = tf32
        try:
            if backend.allow_tf32 != tf32:
                raise RuntimeError("CUDA matmul TF32 request was not applied")
            yield "allow_tf32"
        finally:
            backend.allow_tf32 = previous


def cuda_inputs(torch, fixture, packed, tf32):
    weights, activations, offsets, masks = packed
    with matmul_precision(torch, tf32):
        genetic = weights @ activations
    with matmul_precision(torch, False):
        offset = (offsets @ activations.transpose(1, 2).unsqueeze(-1)).squeeze(-1)
    contributions = genetic + offset.transpose(1, 2)
    result = torch.zeros(fixture.activations.shape, dtype=torch.float32, device="cuda")
    for tile, targets in enumerate(fixture.targets):
        start = int(targets[0])
        result[:, start:start + TILE] += torch.where(
            masks[tile, :, None], contributions[tile].T, 0.0)
    return result


def cuda_assay(fixture, packed, reference, repeats, warmup, rtol, atol):
    try:
        import torch
    except ImportError:
        return {"status": "unavailable", "reason": "PyTorch is not installed; CUDA requested, no CPU fallback"}
    if not torch.cuda.is_available() or torch.version.cuda is None:
        return {"status": "unavailable", "reason": "NVIDIA CUDA device/runtime unavailable; no CPU fallback",
                "torch_version": str(torch.__version__)}
    if os.environ.get("TORCH_ALLOW_TF32_CUBLAS_OVERRIDE") == "1":
        return {"status": "unavailable", "reason": "TORCH_ALLOW_TF32_CUBLAS_OVERRIDE=1 prevents guaranteed FP32 offset matvecs; unset before starting Python"}
    props = torch.cuda.get_device_properties(torch.cuda.current_device())
    report = {"status": "measured", "torch_version": str(torch.__version__),
              "cuda_build_version": torch.version.cuda,
              "precision_control_api": ("fp32_precision" if hasattr(torch.backends.cuda.matmul, "fp32_precision")
                                        else "allow_tf32"),
              "tf32_environment_overrides": {key: os.environ.get(key) for key in
                  ("NVIDIA_TF32_OVERRIDE", "TORCH_ALLOW_TF32_CUBLAS_OVERRIDE")},
              "device": {"index": torch.cuda.current_device(), "name": props.name,
                         "compute_capability": [props.major, props.minor],
                         "total_memory_bytes": props.total_memory,
                         "multiprocessors": props.multi_processor_count},
              "dtype": "float32 master weights, activations, offsets, outputs; bool masks",
              "tensor_core_use": "unproven: TF32 permission is not kernel/counter evidence",
              "offset_matvec_precision": "IEEE FP32 in both modes", "modes": {}}
    report["mode_order"] = "TF32 off then genetic-only on, with separate warmups; not randomized"
    torch.cuda.synchronize()
    start, end = torch.cuda.Event(enable_timing=True), torch.cuda.Event(enable_timing=True)
    wall = time.perf_counter()
    start.record()
    device = [torch.from_numpy(array).to("cuda") for array in vars(packed).values()]
    end.record()
    end.synchronize()
    report["h2d_wall_seconds"] = time.perf_counter() - wall
    report["h2d_event_ms"] = start.elapsed_time(end)
    outputs = {}
    with torch.no_grad(), torch.autocast(device_type="cuda", enabled=False):
        for label, tf32 in (("tf32_off", False), ("tf32_on_genetic_only", True)):
            for _ in range(warmup):
                cuda_inputs(torch, fixture, device, tf32)
            torch.cuda.synchronize()
            event_ms, wall_seconds = [], []
            for _ in range(repeats):
                wall = time.perf_counter()
                start.record()
                output = cuda_inputs(torch, fixture, device, tf32)
                end.record()
                end.synchronize()
                wall_seconds.append(time.perf_counter() - wall)
                event_ms.append(start.elapsed_time(end))
            wall = time.perf_counter()
            outputs[label] = output.cpu().numpy()
            d2h_wall = time.perf_counter() - wall
            report["modes"][label] = {"device_compute_event_ms": event_ms,
                                      "device_compute_event_median_ms": float(np.median(event_ms)),
                                      "compute_wall_seconds": wall_seconds, "d2h_wall_seconds": d2h_wall,
                                      "vs_canonical": drift(outputs[label], reference, rtol, atol)}
    report["tf32_on_vs_identical_tf32_off"] = drift(outputs["tf32_on_genetic_only"], outputs["tf32_off"], rtol, atol)
    return report


def host_info():
    processor = platform.processor()
    if not processor and Path("/proc/cpuinfo").is_file():
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                processor = line.partition(":")[2].strip()
                break
    return {"system": platform.system(), "machine": platform.machine(),
            "processor": processor or "unreported",
            "blas_thread_environment": {key: os.environ.get(key) for key in
                ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS")}}


def run(document, device="cpu", repeats=5, warmup=2, rtol=1e-4, atol=1e-5):
    begin = time.perf_counter()
    fixture = parse_fixture(document)
    identities = verify_identities(fixture)
    wall = time.perf_counter()
    packed = pack_fixture(fixture)
    pack_seconds = time.perf_counter() - wall
    wall = time.perf_counter()
    reference = canonical_inputs(fixture)
    reference_seconds = time.perf_counter() - wall
    oracle = drift(reference, fixture.expected, 0, 0)
    if oracle["bit_different_values"]:
        raise ValueError(f"canonical FP32 oracle differs from native expected_inputs: {oracle}")
    report = {"schema_version": 1, "scope": "independent-window recurrent synapse-sum component only",
              "independent_samples": len(fixture.activations), "neurons": fixture.neurons,
              "canonical_synapses": fixture.weights.size, "recurrent_synapses": fixture.recurrent_ids.size,
              "tiles": len(fixture.ids), "active_tiles_per_sample": fixture.masks.sum(axis=0).tolist(),
              "genetic_gemm_shape": [len(fixture.ids), TILE, TILE, len(fixture.activations)],
              "identity_roundtrips": identities, "canonical_vs_native": oracle,
              "python": platform.python_version(), "numpy": np.__version__,
              "host": host_info(), "pack_wall_seconds": pack_seconds,
              "canonical_python_oracle_wall_seconds": reference_seconds,
              "repeats": repeats, "warmup": warmup,
              "limitations": ["dense genetic/offset split and tile scatter change reduction order",
                              "no production training, recurrence, clamp, BPTT, optimizer or checkpoint parity claim",
                              "component timers exclude native world/trainer; Python oracle is not a native speed baseline",
                              "CUDA event spans include launch gaps; tolerances are diagnostic, not a training qualification",
                              "TF32 flag is not proof of Tensor Core use; profiler kernel/counter evidence required"]}
    if device == "cuda":
        report["cuda"] = cuda_assay(fixture, packed, reference, repeats, warmup, rtol, atol)
    else:
        for _ in range(warmup):
            packed_inputs(fixture, packed)
        times = []
        for _ in range(repeats):
            wall = time.perf_counter()
            output = packed_inputs(fixture, packed)
            times.append(time.perf_counter() - wall)
        report["cpu"] = {"packed_compute_wall_seconds": times,
                         "packed_compute_median_seconds": float(np.median(times)),
                         "vs_canonical": drift(output, reference, rtol, atol)}
    # Deterministic cotangents probe local gradient mapping; these are not training losses.
    cotangent = np.sin(np.arange(fixture.activations.size, dtype=np.float32)).reshape(fixture.activations.shape)
    expected_vjp = canonical_vjp(fixture, cotangent)
    candidate_vjp = packed_vjp(fixture, packed, cotangent)
    report["cpu_local_linear_vjp"] = {name: drift(actual, expected, rtol, atol)
        for name, actual, expected in zip(("canonical_weights", "per_sample_offsets", "activations"),
                                          candidate_vjp, expected_vjp)}
    report["total_sidecar_wall_seconds"] = time.perf_counter() - begin
    report["total_timing_scope"] = "parse/identity/pack/oracle, requested warmups+repeats+copies, CPU local VJP; excludes process/import/file IO"
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("fixture", type=Path, help="fresh disposable public Rust fixture JSON only")
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu")
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--rtol", type=float, default=1e-4)
    parser.add_argument("--atol", type=float, default=1e-5)
    args = parser.parse_args(argv)
    if args.repeats < 1 or args.warmup < 0 or min(args.rtol, args.atol) < 0:
        parser.error("repeats must be positive; warmup/tolerances nonnegative")
    try:
        raw = args.fixture.read_bytes()
        report = run(json.loads(raw), args.device, args.repeats, args.warmup, args.rtol, args.atol)
        report["fixture_sha256"] = hashlib.sha256(raw).hexdigest()
        report["fixture_bytes"] = len(raw)
        print(json.dumps(report, indent=2, allow_nan=False))
        if args.device == "cuda" and report["cuda"]["status"] == "unavailable":
            return 2
        return 0
    except (ValueError, KeyError, TypeError, OSError, RuntimeError) as error:
        print(json.dumps({"status": "rejected", "reason": str(error), "device_requested": args.device}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
