"""Create-only, at-most-four-process native tournament runner. No private inputs included."""
import argparse, concurrent.futures, hashlib, json, subprocess
from pathlib import Path


def run(binary, request, directory):
    directory.mkdir()  # fail rather than resume/overwrite an uncertain run
    request = dict(request, outdir=str((directory / "native").resolve()))
    path = directory / "request.json"
    with path.open("x") as f:
        json.dump(request, f, indent=2)
    with (directory / "stdout.log").open("x") as stdout, (directory / "stderr.log").open("x") as stderr:
        result = subprocess.run([binary, str(path.resolve())], stdout=stdout, stderr=stderr)
    record = {"returncode": result.returncode, "complete": (directory / "native" / "COMPLETE.json").exists()}
    with (directory / "process.json").open("x") as f:
        json.dump(record, f)
    if result.returncode or not record["complete"]:
        raise RuntimeError(f"Native job failed; preserve {directory} for inspection")
    return str(directory)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("binary", type=Path)
    p.add_argument("base_request", type=Path)
    p.add_argument("proposals", type=Path)
    p.add_argument("output", type=Path)
    p.add_argument("--workers", type=int, default=4, choices=range(1, 5))
    a = p.parse_args()
    base = json.loads(a.base_request.read_text())
    proposals = json.loads(a.proposals.read_text())["variants"]
    if len(proposals) != 9 or len({x["id"] for x in proposals}) != 9:
        raise ValueError("Expected nine distinct frozen programs")
    if any(not x["id"] or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789_-" for c in x["id"]) for x in proposals):
        raise ValueError("Unsafe program name")
    if base["horizon"] != 1024 or base["temperature"] != 1:
        raise ValueError("This tournament is fixed to 1024 intervals at T1")
    binary = str(a.binary.resolve(strict=True))
    a.output.mkdir()  # no resume and no clobber
    pins = {str(x): hashlib.sha256(x.read_bytes()).hexdigest() for x in (a.binary, a.base_request, a.proposals)}
    with (a.output / "input-pins.json").open("x") as f:
        json.dump(pins, f, indent=2)
    jobs = [(dict(base, edits=v["weight_edits_bits"], fixture_id=fixture), a.output / f'{v["id"]}-fixture{fixture}') for v in proposals for fixture in (0, 1)]
    errors = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.workers) as pool:
        futures = [pool.submit(run, binary, r, d) for r, d in jobs]
        for future in concurrent.futures.as_completed(futures):
            try:
                future.result()
            except Exception as error:
                errors.append(str(error))
    with (a.output / "driver-result.json").open("x") as f:
        json.dump({"jobs": len(jobs), "failures": errors, "note": "Process closure only; independently audit native boundaries before ranking."}, f, indent=2)
    if errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
