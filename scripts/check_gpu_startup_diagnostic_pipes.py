#!/usr/bin/env python3
"""Run the native diagnostic fixture with an actually full, unread stderr pipe.

Only disposable pipe handles are made nonblocking during setup. The child's
stderr is restored to blocking mode, and its reader remains open and unread.
Python 3.12+ is required for pipe blocking-mode control on Windows.
"""
import argparse
import json
import os
from pathlib import Path
import queue
import subprocess
import threading

ROOT = Path(__file__).resolve().parent.parent
CHILD_ENV = "ALIFE_STARTUP_DIAGNOSTICS_CLOSED_STDERR_CHILD"
FIXTURE = "startup_diagnostics::tests::diagnostic_sink_closed_pipe_does_not_change_exit_status"


def test_binary():
    build = subprocess.run(
        ["cargo", "test", "--locked", "-p", "alife_gpu_backend", "--lib",
         "--no-run", "--message-format=json"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    )
    if build.returncode:
        raise RuntimeError(f"native fixture build failed:\n{build.stderr[-8000:]}")
    paths = set()
    for line in build.stdout.splitlines():
        item = json.loads(line)
        if (item.get("reason") == "compiler-artifact"
                and item.get("target", {}).get("name") == "alife_gpu_backend"
                and item.get("profile", {}).get("test") and item.get("executable")):
            paths.add(item["executable"])
    if len(paths) != 1:
        raise RuntimeError(f"expected exactly one native fixture binary: {paths}")
    return paths.pop()


def fill_pipe(fd):
    os.set_blocking(fd, False)
    total = 0
    for block in (b"x" * 4096, b"x"):
        while True:
            try:
                written = os.write(fd, block)
            except BlockingIOError:
                break
            if written == 0:
                break
            total += written
            if total > 16 * 1024 * 1024:
                raise RuntimeError("fixture pipe did not reach backpressure within 16 MiB")
    if not total:
        raise RuntimeError("fixture pipe was never filled")
    os.set_blocking(fd, True)
    if not os.get_blocking(fd):
        raise RuntimeError("child stderr must use ordinary blocking mode")
    return total


def check(binary, mode):
    reader, writer = os.pipe()
    child = None
    try:
        filled = fill_pipe(writer)
        env = os.environ.copy()
        env[CHILD_ENV] = mode
        child = subprocess.Popen(
            [str(binary), "--exact", FIXTURE, "--nocapture"], cwd=ROOT, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=writer,
        )
        # No read from `reader`, including through communicate: stderr is an
        # explicit fd, not subprocess.PIPE. A closed pipe is a different test.
        try:
            stdout, _ = child.communicate(b"1", timeout=5)
        except subprocess.TimeoutExpired as error:
            child.kill()
            stdout, _ = child.communicate(timeout=5)
            raise RuntimeError(f"{mode}: native handler blocked on full stderr; stdout={stdout!r}") from error
        text = stdout.decode("utf-8")
        required = ["NORMAL_RESULT"]
        if mode != "loss":
            required.append("REQUEST_RETURNED")
        if mode != "request":
            required.append("LOSS_FLAG_SET_CALLBACK_LOCK_RELEASED")
        if child.returncode or any(marker not in text for marker in required):
            raise RuntimeError(f"{mode}: exit={child.returncode}, stdout={text!r}")
        return {"mode": mode, "pipe_prefilled_bytes": filled,
                "stderr_blocking": True, "reader_open_unread": True,
                "exit": child.returncode, "required_markers": required}
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.communicate(timeout=5)
        os.close(writer)
        os.close(reader)


def check_readable(binary):
    env = os.environ.copy()
    env[CHILD_ENV] = "capture"
    child = subprocess.Popen(
        [str(binary), "--exact", FIXTURE, "--nocapture"], cwd=ROOT, env=env,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    lines = queue.Queue(maxsize=1)
    reader = threading.Thread(target=lambda: lines.put(child.stderr.readline()), daemon=True)
    try:
        reader.start()
        child.stdin.write(b"1")
        child.stdin.flush()
        line = lines.get(timeout=5)
        reader.join(timeout=5)
        if reader.is_alive():
            raise RuntimeError("diagnostic reader did not finish")
        expected = (b"gpu_device_failure backend=Dx12 vendor=0x10de device=0x1234 "
                    b"stage=request_device role=other kind=other")
        if line.rstrip(b"\r\n") != expected:
            raise RuntimeError(f"unexpected worker record: {line!r}")
        child.stdin.write(b"2")
        child.stdin.flush()
        stdout, _ = child.communicate(timeout=5)
        if child.returncode or b"NORMAL_RESULT" not in stdout:
            raise RuntimeError(f"readable sink: exit={child.returncode}, stdout={stdout!r}")
        return {"mode": "readable_control", "exit": child.returncode,
                "observed_record": line.decode("ascii").strip()}
    finally:
        if child.poll() is None:
            child.kill()
        child.communicate(timeout=5)
        reader.join(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-binary", type=Path)
    args = parser.parse_args()
    binary = args.test_binary or test_binary()
    results = [check_readable(binary)]
    results.extend(check(binary, mode) for mode in ("request", "loss", "burst"))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
