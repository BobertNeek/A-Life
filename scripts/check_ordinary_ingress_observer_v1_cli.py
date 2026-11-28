"""Artificial no-checkpoint, no-operator-clock schema checks on the existing App CLI."""
from pathlib import Path
from tempfile import TemporaryDirectory
import json
import time
import subprocess
import sys

off, on = [str(Path(s).resolve(strict=True)) for s in sys.argv[1:]]
with TemporaryDirectory(prefix="qualia-private-artificial-ingress-") as name:
    root = Path(name).resolve()
    assert root.stat().st_mode & 0o777 == 0o700
    source, output, manifest = (root / s for s in ("absent-current", "absent-output", "experiment.json"))

    def refuse(exe, cfg):
        manifest.write_text(json.dumps(cfg, sort_keys=True) + "\n")
        p = subprocess.run(
            [exe, "--experiment-manifest", str(root), str(source), str(output), str(manifest)],
            cwd=root, env={"PATH": "/usr/bin:/bin", "LANG": "C"}, stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10, check=False,
        )
        assert p.returncode == 1 and not p.stdout, (p.returncode, p.stdout, p.stderr)
        assert set(root.iterdir()) == {manifest}, list(root.iterdir())
        return p.stderr.decode("utf-8")

    ordinary = {
        "schema": 1, "run_id": "artificial-no-assets", "owner": 7, "scenario": "retained_opening",
        "source_current_sha256": "a" * 64, "intervention": {"kind": "control"},
        "evaluation_nonce": 7, "max_decisions": 2, "max_world_ticks": 2,
        "deadline_unix_seconds": int(time.time()) + 240, "max_output_bytes": 268_435_456,
    }
    # No existing CLI request reaches a present source or device. Exact legacy
    # error equivalence is not claimed to be ordinary runtime byte parity.
    for old in (ordinary, {"schema": 8}, {"schema": 10}):
        a, b = refuse(off, old), refuse(on, old)
        assert a == b and "ordinary ingress recorder feature" not in a, (a, b)

    stock = dict(ordinary, scenario="published_card_case", card_case="r12-word-empty-silence",
                 max_decisions=256, max_world_ticks=256)
    ref = {"schema": 81, "settings": stock, "mode": "reference", "window_start_tick": 12,
           "operator_grant_sha256": "b" * 64}
    assert "ordinary ingress recorder feature is disabled" in refuse(off, ref)
    for invalid in (dict(ref, mode="force_word"), dict(ref, answer=400)):
        assert "unknown" in refuse(on, invalid)
    assert "stand-alone cue-free stock 256/256" in refuse(on, dict(ref, settings=dict(stock, max_world_ticks=8)))
    assert "published stock case" in refuse(on, dict(ref, settings=dict(stock, card_case="not-visible-case")))
    for mode in ("reference", "capture"):
        error = refuse(on, dict(ref, mode=mode))
        assert "No such file" in error and "feature is disabled" not in error, error

print("ordinary_ingress_device_free_cli_and_no_asset_refusals=PASS")
