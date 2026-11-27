#!/usr/bin/env python3
"""Planned built CLI ON/OFF smoke. Only no-device refusals, never an actual Current."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

SCOPE = "qualia.saved_life.n4096_auxiliary_zero.v1"
LEAF = "n4096-auxiliary-admission.v1.json"
PROOF = "qualia.saved_life.n4096_ordinary_zero_proof.v1"
PROOF_LEAF = "n4096-ordinary-zero-admission.v1.json"
ENV = {"PATH": "/usr/bin:/bin", "LC_ALL": "C"}


def raw(x):
    return json.dumps(x, sort_keys=True, separators=(",", ":")).encode()


def file_hash(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def write(path, data):
    path.write_bytes(data)
    path.chmod(0o400)


def run(exe, root, config, expected, marker=False, expected_admission=None, admission_leaf=LEAF):
    with tempfile.TemporaryDirectory(dir=root) as d:
        d = Path(d).resolve(strict=True)
        os.chmod(d, 0o700)
        manifest, source, output = (d / "manifest.json", d / "absent-unused-Current", d / "must-not-exist")
        write(manifest, raw(config))
        if marker:
            write(source, b"Device-free refusal fixture only, not a Current.\n")
        if expected_admission is not None:
            write(d / admission_leaf, raw(expected_admission))
        before = {x.name: file_hash(x) for x in d.iterdir()}
        got = subprocess.run(
            [str(exe), "--experiment-manifest", str(d), str(source), str(output), str(manifest)],
            cwd=d, env=ENV, capture_output=True, timeout=10,
        )
        assert got.returncode == 1 and not got.stdout and expected in got.stderr, (got.returncode, got.stderr)
        assert before == {x.name: file_hash(x) for x in d.iterdir()}, "unexpected write or directory creation"
        return got.returncode, got.stdout, got.stderr


def main():
    off, on = [Path(x).resolve(strict=True) for x in sys.argv[1:]]
    assert file_hash(off) != file_hash(on)
    cap = lambda e: subprocess.run([str(e), "--capabilities"], env=ENV,
        capture_output=True, check=True, timeout=8).stdout
    assert cap(off) == cap(on), "existing shared capabilities changed"
    now = int(time.time())
    adm = {"schema":1, "scope":SCOPE, "run_id":"synthetic-only", "owner":1, "evaluation_nonce":3,
           "deadline_unix_seconds":now+100, "source_current_sha256":"a"*64,
           "evaluator_sha256":file_hash(on), "source_world_json_sha256":"b"*64,
           "source_sleep_journal_sha256":"c"*64, "source_live_topology_sha256":"d"*64,
           "expected_restored_world_signature":[1,2,3,4], "expected_restored_native_snapshot":[5,6,7,8]}
    m = {"schema":12, "scope":SCOPE, "admission_sha256":hashlib.sha256(raw(adm)).hexdigest()}
    with tempfile.TemporaryDirectory() as root:
        run(off, root, m, b"N4096 auxiliary inspection feature is disabled")
        run(on, root, {**m,"source_tick":3074}, b"unknown field")
        run(on, root, {**m,"scope":"untrusted"}, b"unexpected N4096 auxiliary zero envelope")
        run(on, root, m, b"No such file or directory")
        run(on, root, m, b"exact admission digest mismatch", expected_admission={})
        bad = {**adm,"evaluator_sha256":"e"*64}
        run(on, root, {**m,"admission_sha256":hashlib.sha256(raw(bad)).hexdigest()},
            b"evaluator executable mismatch", expected_admission=bad)
        # Exact local typed admission and own ELF hashes reach only missing
        # Current; the same call never sees an actual game or device input.
        run(on, root, m, b"No such file or directory", expected_admission=adm)
        proof = {k:v for k,v in adm.items() if not k.startswith("expected_restored_")}
        proof["scope"] = PROOF
        def proof_envelope(a):
            return {"schema":12, "scope":PROOF, "admission_sha256":hashlib.sha256(raw(a)).hexdigest()}
        e = proof_envelope(proof)
        run(off, root, e, b"N4096 auxiliary inspection feature is disabled")
        # Cross-scope grant/leaf reuse fails; neither this nor a good hash
        # may smuggle width data or provide the proposed observation itself.
        run(on, root, e, b"No such file or directory", expected_admission=proof)
        run(on, root, proof_envelope(adm), b"identity, digest, or deadline refused",
            expected_admission=adm, admission_leaf=PROOF_LEAF)
        for k in ("expected_restored_world_signature", "expected_restored_native_snapshot"):
            for supplied in (None, [1,2,3,4]):
                wrong = {**proof, k:supplied}
                run(on, root, proof_envelope(wrong), b"identity, digest, or deadline refused",
                    expected_admission=wrong, admission_leaf=PROOF_LEAF)
        bad_elf = {**proof, "evaluator_sha256":"e"*64}
        run(on, root, proof_envelope(bad_elf), b"evaluator executable mismatch",
            expected_admission=bad_elf, admission_leaf=PROOF_LEAF)
        run(on, root, e, b"No such file or directory", expected_admission=proof, admission_leaf=PROOF_LEAF)
        old = {"schema":1,"run_id":"ordinary","owner":1,"source_current_sha256":"a"*64,
               "scenario":"retained_opening","intervention":{"kind":"control"},
               "evaluation_nonce":17,"max_decisions":1,"max_world_ticks":1,
               "max_output_bytes":256<<20,"deadline_unix_seconds":now+100}
        gate = b"retained experiment requires explicit software Vulkan mode"
        assert run(off, root, old, gate, marker=True) == run(on, root, old, gate, marker=True)
    print("distinct proof/carrier schema12 scopes, genuine OFF/ON refusals, unchanged ordinary CLI; no Current/device")


if __name__ == "__main__":
    main()
