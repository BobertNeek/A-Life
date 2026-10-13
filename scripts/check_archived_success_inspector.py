"""Exercise native archive admission on real sealed inputs and retained tamper fixtures."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("binary", "source", "manifest", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    original = json.loads(args.manifest.read_bytes())
    assert any(entry["action"] == 210 for entry in original["entries"])
    grab_index = next(i for i, entry in enumerate(original["entries"]) if entry["action"] == 211)
    protected = [args.manifest] + [args.source / name for name in
        ("cycle.json", "trained.alife-foundation", "actor-checkpoint.json", "value-checkpoint.json")]
    digests = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in protected}
    results = []

    def invoke(name, source, manifest, should_pass):
        process = subprocess.run([str(args.binary), str(source), str(manifest)],
                                 capture_output=True, text=True, timeout=900)
        (args.output / (name + ".stdout.json")).write_text(process.stdout)
        (args.output / (name + ".stderr.log")).write_text(process.stderr)
        assert (process.returncode == 0) == should_pass, name + ": " + process.stderr
        if should_pass:
            report = json.loads(process.stdout)
            assert report["checked_examples"] == len(original["entries"])
            assert report["grab_examples"] + report["eating_examples"] == report["checked_examples"]
            assert report["neural_execution"] is False and report["weights_modified"] is False
        results.append({"case": name, "expected_acceptance": should_pass, "exit_code": process.returncode})

    invoke("genuine-bank", args.source, args.manifest, True)
    for name in ("schema", "duplicate", "record-digest", "false-meal", "founder"):
        mutated = json.loads(args.manifest.read_bytes())
        if name == "schema":
            mutated["schema"] = 2
        elif name == "duplicate":
            mutated["entries"].append(mutated["entries"][0])
        elif name == "record-digest":
            mutated["entries"][0]["record_digest"][0] ^= 1
        elif name == "false-meal":
            mutated["entries"][grab_index]["action"] = 210
        else:
            mutated["founder_seed_base"] += 1
        manifest = args.output / (name + ".json")
        manifest.write_text(json.dumps(mutated))
        invoke(name, args.source, manifest, False)
    for name in ("incomplete-handoff", "actor-export-mismatch", "actor-empty-first-moment",
                 "value-empty-parameters", "value-feature-mismatch"):
        source = args.output / name
        source.mkdir()
        for path in protected[1:]:
            shutil.copy2(path, source / path.name)
        checkpoint_name = ("cycle.json" if name == "incomplete-handoff" else
                           "value-checkpoint.json" if name.startswith("value-") else
                           "actor-checkpoint.json")
        path = source / checkpoint_name
        data = json.loads(path.read_bytes())
        if name == "incomplete-handoff":
            data["next_cohort_tick_captured"] = False
        elif name == "actor-export-mismatch":
            data["weights"][0] += .5
        elif name == "actor-empty-first-moment":
            data["first_moment"] = []
        elif name == "value-empty-parameters":
            data["parameters"] = []
        else:
            data["feature_count"] += 1
            # Keep this payload internally well-shaped so only graph feature
            # compatibility can reject it.
            data["parameters"] = [0.0] * ((data["feature_count"] + 1) * 3)
        path.write_text(json.dumps(data))
        invoke(name, source, args.manifest, False)
    assert digests == {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in protected}
    receipt = {"cases": results, "protected_input_sha256": digests,
               "input_bytes_unchanged": True, "neural_execution": False, "weights_modified": False}
    (args.output / "CHECKS.json").write_text(json.dumps(receipt, indent=2))
    print(json.dumps({"checks_passed": len(results), "inputs_unchanged": True, "GPU_started": False}))


if __name__ == "__main__":
    main()
