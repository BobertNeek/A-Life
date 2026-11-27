"""No-device strict CLI probes; neither a checkpoint nor output can exist."""
from pathlib import Path
from tempfile import TemporaryDirectory
import json
import os
import subprocess
import sys
import time

off, on = [str(Path(s).resolve(strict=True)) for s in sys.argv[1:]]
with TemporaryDirectory(prefix="qualia-word-local-admission-no-assets-") as d:
    root = Path(d).resolve()
    source = root / "must-remain-absent-source"
    output = root / "must-remain-absent-output"
    manifest = root / "experiment.json"
    grant = root / "word-local-operator-grant.v1.json"
    assert not any(x.exists() for x in (source, output, grant))
    def refusal(exe, wire):
        manifest.write_text(json.dumps(wire, sort_keys=True)+"\n")
        env = {k: os.environ[k] for k in ("PATH", "HOME", "LANG") if k in os.environ}
        env["LC_ALL"] = "C"
        run = subprocess.run([exe, "--experiment-manifest", str(root), str(source),
              str(output), str(manifest)], cwd=root, env=env, stdin=subprocess.DEVNULL,
              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=12, check=False)
        assert run.returncode == 1 and not run.stdout, (run.returncode, run.stdout, run.stderr)
        assert not any(x.exists() for x in (source, output, grant)), list(root.iterdir())
        return run.stderr.decode("utf-8")
    stock = dict(schema=1, run_id="synthetic-no-assets", owner=3, source_current_sha256="b"*64,
       scenario="retained_opening", intervention={"kind":"control"}, evaluation_nonce=7,
       max_decisions=2, max_world_ticks=3, deadline_unix_seconds=int(time.time())+300,
       finalization_deadline_unix_seconds=None, max_output_bytes=8388608)
    for old in (stock, dict(stock, max_decisions=0), {"schema":8}):
        before = refusal(off, old)
        after = refusal(on, old)
        assert before == after and "Word-local" not in after, (before, after)
    assert "Word-local grounding feature is disabled" in refusal(off, {"schema":10})
    settings = dict(stock, scenario="published_card_case", card_case="no-real-case-created",
      max_decisions=256,max_world_ticks=256)
    wb = dict(schema=10,settings=settings,mode={"mode":"observer_only"},operator_grant_sha256="a"*64)
    assert "unknown field" in refusal(on, dict(wb, selector_override=400))
    assert "unsupported Word-local scope/settings" in refusal(on, dict(wb, settings=dict(settings,max_decisions=1)))
    assert "unsupported Word-local scope/settings" in refusal(on, dict(wb, settings=dict(settings,intervention={"kind":"indexed_near_field"})))
    assert "Word-local grounding feature is disabled" in refusal(off, wb)
    e = refusal(on, wb)
    assert ("No such file" in e) and ("Word-local grounding feature is disabled" not in e), e
    x=refusal(on, dict(wb, mode={"mode":"signed_edit","weights":{"coefficients":[129.,0.,0.,0.,0.,0.]}}))
    assert ("perception" in x.lower()) and "No such file" not in x, x
print("word_local_off_and_on_strict_refusals_and_legacy_unavailable_input_parity=PASS")
print("asset_free_absent_current_output_and_operator_grant=PASS")
