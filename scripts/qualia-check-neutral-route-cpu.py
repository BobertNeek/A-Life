#!/usr/bin/env python3
"""Synthetic exact shared CLI negatives; pass prebuilt OFF/ON executables only."""
from pathlib import Path
from tempfile import TemporaryDirectory
import argparse
import hashlib
import json
import os
import subprocess
import time


def sha(path):
    with path.open("rb") as fp:
        return hashlib.file_digest(fp, "sha256").hexdigest()


def main():
    ap = argparse.ArgumentParser()
    for name in ("off", "on"):
        ap.add_argument(f"--{name}", required=True, type=Path)
        ap.add_argument(f"--{name}-sha256", required=True)
    a = ap.parse_args()
    executables = {name: getattr(a, name).resolve(strict=True) for name in ("off", "on")}
    assert not a.off.is_symlink() and not a.on.is_symlink()
    assert executables["off"] != executables["on"]
    for name, elf in executables.items():
        assert elf.is_file() and os.access(elf, os.X_OK)
        assert sha(elf) == getattr(a, name + "_sha256")

    with TemporaryDirectory(prefix="neutral-route-device-free-") as tmp:
        root = Path(tmp).resolve()
        source, output, manifest = root / "Current", root / "must-not-exist", root / "manifest.json"
        source.write_bytes(b"not a real Current; scope test only\n")
        source_hash = sha(source)
        wrong_hash = "0" * 64
        assert source_hash != wrong_hash
        clean_env = {
            "PATH": "/usr/bin:/bin", "HOME": "/tmp", "LC_ALL": "C",
            "ALIFE_EXPLICIT_SOFTWARE_RETENTION_TEST": "1",
            "ALIFE_SLM_PRIOR": "off", "ALIFE_SLM_PRIOR_BACKEND": "off",
        }

        def base():
            return {
                "schema": 19, "run_id": "neutral_cpu_parser", "owner": 123,
                "source_current_sha256": wrong_hash, "scenario": "retained_opening",
                "intervention": {"kind": "neutral_route_observation_v1", "observation": {
                    "source_world_tick": 100, "first_postcommit_world_offset": 6,
                    "postcommit_boundary_count": 3,
                }},
                "evaluation_nonce": 7, "max_decisions": 12, "max_world_ticks": 12,
                "deadline_unix_seconds": int(time.time()) + 120, "max_output_bytes": 268435456,
            }

        total = 0

        def refusal(mode, value, reason, **changed_environment):
            nonlocal total
            wire = value if isinstance(value, str) else json.dumps(value, separators=(",", ":"))
            manifest.write_text(wire, encoding="utf-8")
            p = subprocess.run([str(executables[mode]), "--experiment-manifest", str(root),
                                str(source), str(output), str(manifest)],
                               env=clean_env | changed_environment, capture_output=True,
                               text=True, timeout=8, check=False)
            assert p.returncode != 0 and reason in p.stderr, (mode, reason, p.stdout, p.stderr)
            assert not output.exists() and sha(source) == source_hash
            total += 1

        refusal("off", base(), "invalid or unsupported typed settings")
        refusal("on", base(), "source Current hash mismatch")
        ordinary = base()
        ordinary["schema"] = 1
        ordinary["intervention"] = {"kind": "control"}
        for mode in ("off", "on"):
            refusal(mode, ordinary, "source Current hash mismatch")
        for name, value, reason in (
            ("schema", 1, "unsupported manifest schema"),
            ("card_case", "not-a-case", "unsupported manifest schema"),
            ("max_world_ticks", 13, "requires unchanged retained opening and bounded cuts"),
            ("max_decisions", 4, "requires unchanged retained opening and bounded cuts"),
            ("scenario", "cue_free_food_placement", "requires unchanged retained opening and bounded cuts"),
        ):
            invocation = base()
            invocation[name] = value
            refusal("on", invocation, reason)
        for name, value in (
            ("ALIFE_EXPLICIT_FIXED_HOST_PRESSURE_NS", "0"),
            ("ALIFE_EXPLICIT_RETENTION_ARCHIVE_TEST", "1"),
            ("ALIFE_EXPLICIT_SOFTWARE_RETENTION_TEST", "0"),
        ):
            refusal("on", base(), "requires natural-pressure software mode with no archive override",
                    **{name: value})
        for field in ("arm", "fixture", "direct_weights", "positive_reward_scalar", "forced_motor_candidate"):
            invocation = base()
            invocation["intervention"]["observation"][field] = 1
            refusal("on", invocation, "invalid or unsupported typed settings")
        duplicate = json.dumps(base(), separators=(",", ":")).replace(
            '"source_world_tick":100', '"source_world_tick":100,"source_world_tick":101')
        refusal("on", duplicate, "invalid or unsupported typed settings")
        for first, count in ((0, 1), (12, 2), (1, 4)):
            invocation = base()
            invocation["intervention"]["observation"]["first_postcommit_world_offset"] = first
            invocation["intervention"]["observation"]["postcommit_boundary_count"] = count
            refusal("on", invocation, "does not match the installed neural decision evidence")
    print(f"{total} synthetic shared CLI refusals; no Current decode/output, GPU, or World execution")


if __name__ == "__main__":
    main()
