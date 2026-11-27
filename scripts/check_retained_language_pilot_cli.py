"""Exercise actual manifest CLI admission, without a world, checkpoint, or device."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from tempfile import TemporaryDirectory
import time


def checked(test, detail):
    if not test:
        raise SystemExit(f"qualification failed: {detail}")


def sha(blob):
    return hashlib.sha256(blob).hexdigest()


def wire(obj):
    return json.dumps(obj, separators=(",", ":")).encode()


def snapshot(directory):
    # Check file bytes and symlink text without following a possible new link.
    result = {}
    for p in directory.iterdir():
        result[p.name] = (
            ("link", os.readlink(p)) if p.is_symlink() else
            ("file", sha(p.read_bytes())) if p.is_file() else ("directory",)
        )
    return result


def main(off, on, reviewed_source):
    off, on, reviewed_source = (Path(x).resolve(strict=True) for x in (off, on, reviewed_source))
    rust = reviewed_source.read_text()
    def const(name):
        match = re.search(r'^const ' + name + r': &str = "([^"]+)";$', rust, re.M)
        checked(match is not None, f"missing reviewed contract constant {name}")
        return match[1]

    fixture_id, current_pin, plan_pin = (const(k) for k in ("ID", "SOURCE", "PLAN"))
    with on.open("rb") as f:
        elf_pin = hashlib.file_digest(f, "sha256").hexdigest()
    with off.open("rb") as f:
        checked(elf_pin != hashlib.file_digest(f, "sha256").hexdigest(), "feature outputs alias")
    env = {"PATH": "/usr/bin:/bin", "LC_ALL": "C"}  # No Vulkan/retention opt-in, credentials, or ambient config.
    old_cap = subprocess.run([off, "--capabilities"], env=env, capture_output=True, timeout=10, check=True)
    new_cap = subprocess.run([on, "--capabilities"], env=env, capture_output=True, timeout=10, check=True)
    checked(old_cap.stdout == new_cap.stdout and old_cap.stderr == new_cap.stderr == b"",
            "old --capabilities bytes changed")
    checked(json.loads(old_cap.stdout)["kind"] == "saved-life-retention-v2", "wrong executable")
    print("ON/OFF old --capabilities exact byte parity")

    valid_schema_error = "retained experiment requires explicit software Vulkan mode"
    missing_file = "No such file or directory"
    legal = ([9, 1], [9, 14], [14, 1], [6, 1])
    rubric = wire({"ci_admission_probe_only": True, "contains_behavior_expectation": False})
    sentinel = b"Inert CI path-boundary marker; not checkpoint data.\n"

    def probe(name, expected, *, exe=on, layout="a", pair=legal[0], change=None,
              old=None, inert=False, path_case=None):
        with TemporaryDirectory(prefix="qualia-ci-no-checkpoint-") as tmp:
            root = Path(tmp).resolve(strict=True)
            source, output = root / "current-boundary-marker", root / "unused-output"
            manifest_path = root / "synthetic.json"
            admission_path = root / "language-pilot-admission.v2.json"
            rubric_path = root / "language-pilot-rubric.v2.json"
            rubric_path.write_bytes(rubric)
            arm = {"layout": layout, "tokens": list(pair)}
            admission = dict(schema="qualia.saved_life.language_pilot_admission.v2",
                             fixture_id=fixture_id, source_current_sha256=current_pin,
                             fixture_plan_sha256=plan_pin, evaluator_sha256=elf_pin,
                             rubric_sha256=sha(rubric), cell=dict(arm))
            spec = dict(schema=5, fixture_id=fixture_id, run_id="ci-probe",
                        source_current_sha256=current_pin, cell=dict(arm),
                        deadline_unix_seconds=int(time.time()) + 90)
            if change:
                change(spec, admission)
            admission_path.write_bytes(wire(admission))
            spec.setdefault("admission_sha256", sha(admission_path.read_bytes()))
            if old:
                spec = dict(schema=1, run_id="ci-probe", owner=1, source_current_sha256="a" * 64,
                            scenario=old, intervention={"kind": "control"}, evaluation_nonce=17,
                            max_decisions=16, max_world_ticks=16, max_output_bytes=256 << 20,
                            deadline_unix_seconds=int(time.time()) + 90)
                if old == "published_card_case":
                    spec["card_case"] = "r2-word-retreat-food"  # Disclosed legacy registry; no scene data.
            manifest_path.write_bytes(wire(spec))
            if inert:
                source.write_bytes(sentinel)
            if path_case == "rubric_symlink":
                real_rubric = root / "inert-rubric-copy.json"
                rubric_path.rename(real_rubric)
                rubric_path.symlink_to(real_rubric)
            if path_case == "existing_output":
                output.mkdir()
            before = snapshot(root)
            run = subprocess.run([exe, "--experiment-manifest", root, source, output, manifest_path],
                                 env=env, text=True, capture_output=True, timeout=12)
            checked(run.returncode == 1 and not run.stdout and expected in run.stderr,
                    f"{name}: code={run.returncode}, stdout={run.stdout!r}, stderr={run.stderr!r}")
            checked(before == snapshot(root), f"{name}: input/output artifact mutated or created")
            print(f"{name}: verified compiled refusal ({expected}); no changes")
            return run.returncode, run.stdout, run.stderr

    # Both boundary checks are necessary: a random missing admission file could
    # otherwise masquerade as a valid schema that got as far as missing Current.
    for layout in ("a", "b"):
        for pair in legal:
            probe(f"schema5 {layout} {pair} admitted, absent Current", missing_file, layout=layout, pair=pair)
            probe(f"schema5 {layout} {pair} admitted, inert marker", valid_schema_error,
                  layout=layout, pair=pair, inert=True)
    probe("reject superseded v1 pilot fixture", "unadmitted language pilot identity or Player pair",
          change=lambda m, _: m.update(fixture_id="v2-language-get-food-crossed-ball-v1"))
    probe("reject old admission schema even in the new filename",
          "language pilot source, cell, rubric, or executable does not match admission",
          change=lambda _, a: a.update(schema="qualia.saved_life.language_pilot_admission.v1"))
    probe("reject extra answer field", "unknown field `expected_entity`",
          change=lambda m, _: m["cell"].update(expected_entity=2))
    probe("reject unregistered pair", "unadmitted language pilot identity or Player pair",
          change=lambda m, a: (m["cell"].update(tokens=[9, 2]), a["cell"].update(tokens=[9, 2])))
    probe("reject altered source pin", "unadmitted language pilot identity or Player pair",
          change=lambda m, _: m.update(source_current_sha256="0" * 64))
    probe("reject wrong outer admission hash", "language pilot operator admission hash mismatch",
          change=lambda m, _: m.update(admission_sha256="0" * 64))
    for field in ("fixture_plan_sha256", "evaluator_sha256", "rubric_sha256"):
        probe(f"reject substituted {field}", "language pilot source, cell, rubric, or executable does not match admission",
              change=lambda _, a, field=field: a.update({field: "0" * 64}))
    probe("reject different admission cell", "language pilot source, cell, rubric, or executable does not match admission",
          change=lambda _, a: a["cell"].update(layout="b"))
    probe("reject symlink admission rubric", "language pilot requires canonical local admission leaves",
          path_case="rubric_symlink")
    probe("reject expired bounds before opening", "invalid saved-life retention bounds",
          change=lambda m, _: m.update(deadline_unix_seconds=1))
    probe("reject preexisting output", "experiment output already exists", inert=True, path_case="existing_output")
    probe("feature-off cannot resolve schema5", "experiment compatibility: invalid or unsupported typed settings",
          exe=off, inert=True)
    probe("feature-on legacy scenario cannot opt into pilot", "language pilot requires a separate admitted manifest profile",
          old="language_pilot_v2", inert=True)
    probe("feature-off has no pilot scenario", "unknown variant `language_pilot_v2`",
          exe=off, old="language_pilot_v2", inert=True)
    for scenario in ("retained_opening", "published_card_case"):
        a = probe(f"feature-off legacy {scenario}", valid_schema_error, exe=off, old=scenario, inert=True)
        b = probe(f"feature-on legacy {scenario}", valid_schema_error, old=scenario, inert=True)
        checked(a == b, f"legacy {scenario} CLI output changed")
    print("Detached schema5 source-compiled admission only; no world, GPU, official score, or behavior parity claimed.")


if __name__ == "__main__":
    main(*sys.argv[1:])
