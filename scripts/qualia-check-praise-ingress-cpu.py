#!/usr/bin/env python3
"""Review/CI harness only. Two already-built exact OFF/ON binaries; never run a valid Current."""
from pathlib import Path
from tempfile import TemporaryDirectory
import argparse, hashlib, json, os, subprocess, time

def main() -> None:
    ap=argparse.ArgumentParser()
    for profile in ("off","on"):
        ap.add_argument(f"--{profile}", type=Path, required=True)
        ap.add_argument(f"--{profile}-sha256", required=True)
    a=ap.parse_args()
    binaries = {"off": a.off.resolve(strict=True), "on": a.on.resolve(strict=True)}
    assert not a.off.is_symlink() and not a.on.is_symlink() and binaries["on"]!=binaries["off"]
    for key,b in binaries.items():
        assert b.is_file() and os.access(b,os.X_OK)
        with b.open("rb") as f:
            assert hashlib.file_digest(f,"sha256").hexdigest() == getattr(a,key+"_sha256")
    # Never use real checkpoint bytes, paths, settings/env or inherited prior flags.
    env={"PATH":"/usr/bin:/bin","HOME":"/tmp","LC_ALL":"C",
        "ALIFE_EXPLICIT_SOFTWARE_RETENTION_TEST":"1",
        "ALIFE_SLM_PRIOR":"off","ALIFE_SLM_PRIOR_BACKEND":"off"}
    with TemporaryDirectory(prefix="qualia-praise-ingress-device-free-") as temp:
        root=Path(temp).resolve()
        source=root/"Current"
        source.write_bytes(b"not a saved life; CPU-only parser refusal fixture\n")
        fake_pin="0"*64
        assert hashlib.sha256(source.read_bytes()).hexdigest()!=fake_pin
        def manifest(arm):
            return {"schema":18,"run_id":"cpu-praise-parse","owner":3,"scenario":"retained_opening",
                "source_current_sha256":fake_pin,"evaluation_nonce":7,"max_decisions":4,"max_world_ticks":4,
                "deadline_unix_seconds":int(time.time())+60,"max_output_bytes":268435456,
                "intervention":{"kind":"praise_calibration_v1","calibration":{"arm":arm,
                    "expected_start_world_tick":100,"hand_world_position":[0.0,1.0,2.0]}}}
        def assert_refusal(mode, value, message, extra=None):
            path=root/"probe.json"; output=root/"must-remain-absent"
            path.write_text(json.dumps(value))
            p=subprocess.run([str(binaries[mode]),"--experiment-manifest",str(root),str(source),str(output),str(path)],
                env=env|(extra or {}),capture_output=True,text=True,timeout=8)
            assert p.returncode and message in p.stderr and not output.exists(), (mode,p.returncode,p.stdout,p.stderr)
            assert hashlib.sha256(source.read_bytes()).hexdigest()!=fake_pin
        total=0
        for arm in ("quiet","praise","delayed_yoked"):
            assert_refusal("off",manifest(arm),"invalid or unsupported typed settings");total+=1
            assert_refusal("on",manifest(arm),"source Current hash mismatch");total+=1
        for name,value in (("schema",1),("max_decisions",3),("max_world_ticks",5),
                           ("scenario","cue_free_food_placement"),("card_case","r2-word-approach-food")):
            v=manifest("praise");v[name]=value
            reason="unsupported manifest schema" if name in ("schema","card_case") else "four bounded cuts"
            assert_refusal("on",v,reason);total+=1
        for name,value in (("ALIFE_EXPLICIT_FIXED_HOST_PRESSURE_NS","0"),("ALIFE_EXPLICIT_RETENTION_ARCHIVE_TEST","1")):
            assert_refusal("on",manifest("delayed_yoked"),"refuses controlled pressure or archive mode",{name:value});total+=1
        v=manifest("praise");v["intervention"]["calibration"]["clicked_at"]=100
        assert_refusal("on",v,"invalid or unsupported typed settings");total+=1
        v=manifest("praise");v["intervention"]["calibration"]["dose"]=1
        assert_refusal("on",v,"invalid or unsupported typed settings");total+=1
    print(f"{total} actual CLI device-free refusals; no real Current, output, GPU or World used")

if __name__=="__main__":
    main()
