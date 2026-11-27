"""Inert CLI refusal qualification. Never supplies a checkpoint or software Vulkan opt-in."""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
from tempfile import TemporaryDirectory
import time


SCOPED = "qualia.saved_life.language_composition_"
ADMISSION = "language-composition-admission.v3.json"
FIXTURE = "language-composition-fixture.v3.json"
RUBRIC = "language-composition-rubric.v3.json"
GRANT = "language-composition-operator-grant.v3.json"
GEOMETRY = "language-composition-geometry-proof.v3.json"
PAIR = "language-composition-opening-pair-proof.v1.json"
SOFTWARE_OFF = "retained experiment requires explicit software Vulkan mode"
NO_FILE = "No such file or directory"
DISABLED = "v3 language composition feature is disabled"
CURRENT = "a" * 64  # Inert synthetic identifier; never a Current digest or path.
VIRGIN = "absent-no-checkpoint"
MARKER = b"Device-free boundary test marker, not a checkpoint.\n"
ENV = {"PATH": "/usr/bin:/bin", "LC_ALL": "C"}


def require(ok, why):
    if not ok:
        raise SystemExit(f"qualification failed: {why}")


def wire(value):
    return json.dumps(value, separators=(",", ":"), sort_keys=True).encode()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def pins(exe):
    with exe.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def fingerprint(root):
    out = {}
    for p in root.rglob("*"):
        rel = p.relative_to(root).as_posix()
        mode = p.lstat().st_mode
        if stat.S_ISLNK(mode):
            out[rel] = ("symlink", mode, os.readlink(p))
        elif stat.S_ISREG(mode):
            out[rel] = ("file", mode, sha(p.read_bytes()))
        else:
            out[rel] = ("dir-or-special", mode)
    return out


def call(root, exe, manifest, name, expected, *, marker=False, preexisting=False):
    p = root / "manifest.json"
    p.write_bytes(wire(manifest))
    src, out = root / VIRGIN, root / "unused-output"
    if marker:
        src.write_bytes(MARKER)
    if preexisting:
        out.mkdir()
    before = fingerprint(root)
    ctl = subprocess.run(
        [exe, "--experiment-manifest", root, src, out, p],
        cwd=root, env=ENV, capture_output=True, timeout=10,
    )
    result = ctl.returncode, ctl.stdout, ctl.stderr
    require(result[0] == 1 and result[1] == b"" and expected.encode() in result[2],
            f"{name}: wrong device-free refusal return={result!r}")
    require(before == fingerprint(root), f"{name}: input or output mutation")
    print(f"{name}: exact compiled refusal, no output or input change")
    return result


def fixture_id(scope):
    return "synthetic-schema6-" + scope


def v3(exe, name, expected, *, phase="geometry", layout="a", edit=None, marker=True,
       preexisting=False, symlink=False, geometry_wire=None):
    with TemporaryDirectory(prefix="qualia-v3-no-checkpoint-") as tmp:
        path = Path(tmp).resolve(strict=True)
        a = {"schema": SCOPED + "admission.v3", "phase": phase, "run_id": "inert-case",
             "fixture_id": fixture_id("no-live-data"), "source_current_sha256": CURRENT,
             "evaluator_sha256": pins(exe), "cell": {"layout": layout, "player_tokens": [] if phase == "geometry" else [9]}}
        m = {k:a[k] for k in ("phase","run_id","fixture_id","source_current_sha256","cell")}
        m["cell"] = dict(m["cell"])
        m["schema"] = 6
        m["deadline_unix_seconds"] = int(time.time()) + 90
        f = {"schema": SCOPED + "fixture.v3", "fixture_id":m["fixture_id"], "source_current_sha256":CURRENT,
             "owner":71,"tick":199,"world_seed":928,"object_count":3,"nonce":988,
             "props_food_ball":[27,28],"berths_forward_left_mm":[[1700,1500],[1800,-1500]]}
        rubric = {"synthetic_device_free_gate":True}
        grant = {"synthetic_device_free_gate":True}
        a["fixture_sha256"] = sha(wire(f))
        a["rubric_sha256"] = sha(wire(rubric))
        a["operator_grant_sha256"] = sha(wire(grant))
        setup = {"inert": True}
        geo = {"schema":SCOPED+"geometry.v3","fixture_sha256":a["fixture_sha256"],
               "source_current_sha256":CURRENT,"evaluator_sha256":a["evaluator_sha256"],
               "layout":layout,"owner":f["owner"],"world_tick":f["tick"],
               "accepted_decisions":0,"world_ticks":0,"backend_constructed":False,
               "runtime_or_queue_constructed":False,"player_utterances_emitted":0,
               "setup_sha256":sha(wire(setup)),"setup":setup}
        if phase != "geometry":
            a["geometry_proof_sha256"] = sha(wire(geo))
        # Synthetic *invalid* pair cases only, never an independent qualified pair
        # or a positive TTL admission. Always refuse before the absent input.
        if edit:
            grant = edit(m, a, f, geo, grant, path) or grant
        if a.get("geometry_proof_sha256") is not None:
            gbytes = wire(geo)
            if geometry_wire is not None:
                gbytes = geometry_wire(geo)
                # Outer proof follows these literal bytes even in hostile controls.
                a["geometry_proof_sha256"] = sha(gbytes)
            (path / GEOMETRY).write_bytes(gbytes)
        for leaf, obj in ((ADMISSION,a),(RUBRIC,rubric),(FIXTURE,f),(GRANT,grant)):
            (path / leaf).write_bytes(wire(obj))
        if symlink:
            r = path / RUBRIC
            p = path / "synthetic-rubric-target.json"
            r.rename(p)
            r.symlink_to(p)
        m.setdefault("admission_sha256", sha(wire(a)))
        return call(path, exe, m, name, expected, marker=marker, preexisting=preexisting)


def ttl_bad_case(kind):
    def edit(m,a,f,geo,original,root):
        fields = ("fixture_id","source_current_sha256","evaluator_sha256","fixture_sha256")
        base = {k:a[k] for k in fields}
        phase_pins = dict(geometry_rubric_sha256="0"*64, opening_rubric_sha256="5"*64,
                          independent_opening_pair_closure_sha256="6"*64)
        if kind == "same-phase-rubric":
            phase_pins["geometry_rubric_sha256"] = phase_pins["opening_rubric_sha256"]
        if kind == "ttl-reuses-geometry-rubric":
            phase_pins["geometry_rubric_sha256"] = a["rubric_sha256"]
        if kind == "ttl-reuses-opening-rubric":
            phase_pins["opening_rubric_sha256"] = a["rubric_sha256"]
        p = dict(base, **phase_pins, schema=SCOPED+"opening_pair_proof.v2",phase="opening",owner=f["owner"],
                 opening_tick=f["tick"], opening_player_tokens=[9])
        def arm(layout, number, geohash):
            unique = str(number)*64
            return {"layout":layout,"run_id":"synthetic-"+layout,"accepted_decisions":1,"world_ticks":1,
                    "manifest_sha256":unique,"admission_sha256":unique,"operator_grant_sha256":"e"*64,
                    "geometry_proof_sha256":geohash,"retention_sha256":unique,"rows_sha256":unique,
                    "final_current_sha256":unique}
        p["layouts"] = [arm("a",1,a["geometry_proof_sha256"]),arm("b",2,"b"*64)]
        p["layouts"][1]["accepted_decisions"] = 0
        if kind == "bad-pair-alias":
            p["layouts"][1] = dict(p["layouts"][0])
        if kind == "bad-pair-parent-id":
            p["evaluator_sha256"] = "9" * 64
        if kind == "old-v1-proof":
            p["schema"] = SCOPED + "opening_pair_proof.v1"
        if kind == "wrong-opening-rubric":
            p["opening_rubric_sha256"] = "3"*64
        if kind == "wrong-geometry-rubric":
            p["geometry_rubric_sha256"] = "2"*64
        if kind == "wrong-independent-closure":
            p["independent_opening_pair_closure_sha256"] = "4"*64
        wire_proof = wire(p)
        pin = ("f" * 64) if kind == "bad-pair-hash" else sha(wire_proof)
        g = dict(base, **phase_pins, schema=SCOPED+"ttl_operator_grant.v2", phase="ttl", run_id=m["run_id"],
                 rubric_sha256=a["rubric_sha256"],cell=m["cell"],
                 geometry_proof_sha256=a["geometry_proof_sha256"], opening_pair_proof_sha256=pin)
        if kind == "old-v1-grant":
            g["schema"] = SCOPED + "ttl_operator_grant.v1"
        if kind == "wrong-operator-grant":
            g["opening_pair_proof_sha256"] = "8"*64
        if kind != "absent-pair":
            (root / PAIR).write_bytes(wire_proof)
        a["opening_pair_proof_sha256"] = pin
        a["operator_grant_sha256"] = sha(wire(g))
        return g
    return edit


def lexical_geometry(mode):
    # Artificial default serde_json 1.0.150 non-float_roundtrip boundary:
    # converting the mantissa integer then dividing rounds one ULP differently.
    compact = br'{"draw":123.45678901234567,"label":"  \" \\ \t \u0020 ","nested":{"x":-0.0,"y":7e-10}}'
    pretty = br'''{
      "draw": 123.45678901234567,
      "label": "  \" \\ \t \u0020 ",
      "nested": { "x": -0.0, "y": 7e-10 }
    }'''
    def raw(geo):
        geo["setup_sha256"] = sha(compact)
        h = wire(geo)
        marker = b'"setup":{"inert":true}'
        require(h.count(marker) == 1, "wrong synthetic geometry marker")
        baseline = h.replace(marker, b'"setup":' + pretty)
        if mode == "good":
            return baseline
        if mode == "tamper":
            mutated = baseline.replace(b"123.45678901234567", b"123.45678901234566")
            require(len(mutated) == len(baseline) and sum(x != y for x,y in zip(mutated,baseline)) == 1,
                    "synthetic tamper must change exactly one signed byte")
            return mutated
        if mode == "duplicate":
            return h.replace(marker, br'"setup":{},"\u0073etup":' + pretty)
        raise AssertionError("unknown exact lexical negative")
    return raw

def change_geo(m,a,f,geo,grant,root):
    geo["layout"] = "b"
    a["geometry_proof_sha256"] = sha(wire(geo))


def v2(exe, name, expected, contract, *, layout="a", words=(9,1), mutation=None, marker=True):
    with TemporaryDirectory(prefix="qualia-v2-no-checkpoint-") as tmp:
        path = Path(tmp).resolve(strict=True)
        fixture, source, plan = contract
        rubric = wire({"synthetic_device_free_legacy":True})
        cell = {"layout":layout,"tokens":list(words)}
        a = {"schema":"qualia.saved_life.language_pilot_admission.v2","fixture_id":fixture,
             "source_current_sha256":source,"fixture_plan_sha256":plan,"evaluator_sha256":pins(exe),
             "rubric_sha256":sha(rubric),"cell":dict(cell)}
        m = {"schema":5,"fixture_id":fixture,"run_id":"legacy-inert",
             "source_current_sha256":source,"cell":dict(cell),"deadline_unix_seconds":int(time.time())+90}
        if mutation:
            mutation(m,a)
        (path / "language-pilot-admission.v2.json").write_bytes(wire(a))
        (path / "language-pilot-rubric.v2.json").write_bytes(rubric)
        m.setdefault("admission_sha256", sha(wire(a)))
        return call(path, exe, m, name, expected, marker=marker)


def v1(exe, scenario, expected, case=None):
    with TemporaryDirectory(prefix="qualia-v1-no-checkpoint-") as tmp:
        path = Path(tmp).resolve(strict=True)
        m = {"schema":1,"run_id":"legacy-inert","owner":1,"source_current_sha256":CURRENT,
             "scenario":scenario,"intervention":{"kind":"control"},"evaluation_nonce":17,
             "max_decisions":16,"max_world_ticks":16,"max_output_bytes":256 << 20,
             "deadline_unix_seconds":int(time.time())+90}
        if case is not None:
            m["card_case"] = case
        return call(path, exe, m, "unchanged schema1 " + scenario, expected, marker=True)


def main(off_path, on_path, v2_source):
    off, on, source = (Path(p).resolve(strict=True) for p in (off_path, on_path, v2_source))
    require(pins(off) != pins(on), "feature builds alias")
    caps = [subprocess.run([p,"--capabilities"],env=ENV,capture_output=True,timeout=10,check=True)
            for p in (off,on)]
    require(caps[0].stdout == caps[1].stdout and caps[0].stderr == caps[1].stderr == b"",
            "legacy capability output changed")
    require(json.loads(caps[1].stdout)["kind"] == "saved-life-retention-v2", "not stock shared CLI")
    rust = source.read_text()
    def constant(name):
        m = re.search(r'^const ' + name + r': &str = "([^"]+)";$', rust, re.M)
        require(m is not None, f"old source contract missing {name}")
        return m[1]
    contract = tuple(constant(name) for name in ("ID","SOURCE","PLAN"))
    for s, case in (("retained_opening",None),("published_card_case","r2-word-retreat-food")):
        require(v1(off,s,SOFTWARE_OFF,case) == v1(on,s,SOFTWARE_OFF,case), "schema1 exact stderr changed")
    for l in ("a","b"):
        for words in ((9,1),(9,14),(14,1),(6,1)):
            for marker, message in ((False,NO_FILE),(True,SOFTWARE_OFF)):
                run = lambda e:v2(e,"unchanged schema5",message,contract,layout=l,words=words,marker=marker)
                require(run(off) == run(on), "old schema5 boundary changed")
    for old, mutation in (
        ("unadmitted language pilot identity or Player pair",
            lambda m,a:m.update(fixture_id="v2-language-get-food-crossed-ball-v1")),
        ("language pilot source, cell, rubric, or executable does not match admission",
            lambda m,a:a.update(schema="qualia.saved_life.language_pilot_admission.v1")),
    ):
        require(v2(off,"unchanged old schema5 refusal",old,contract,mutation=mutation)
             == v2(on,"unchanged old schema5 refusal",old,contract,mutation=mutation),
             "old refusal bytes differ")
    for phase in ("geometry","opening","ttl"):
        v3(off,"new phase feature-off " + phase,DISABLED,phase=phase)
    for phase in ("geometry","opening"):
        for layout in ("a","b"):
            v3(on,"structural only " + phase + "/" + layout,NO_FILE,phase=phase,layout=layout,marker=False)
            v3(on,"structural marker only " + phase + "/" + layout,SOFTWARE_OFF,phase=phase,layout=layout)
    v3(on,"unknown phase","unknown variant `sequence`",phase="opening",
       edit=lambda m,*_:m.update(phase="sequence"))
    v3(on,"answer field","unknown field `expected_target`",phase="opening",
       edit=lambda m,*_:m["cell"].update(expected_target=1))
    v3(on,"nonempty geometry","v3 manifest phase, scope or genuine Player token vector refused",
       edit=lambda m,*_:m["cell"].update(player_tokens=[9]))
    v3(on,"padded opening","v3 manifest phase, scope or genuine Player token vector refused",phase="opening",
       edit=lambda m,*_:m["cell"].update(player_tokens=[9,1]))
    v3(on,"wrong word","v3 manifest phase, scope or genuine Player token vector refused",phase="opening",
       edit=lambda m,*_:m["cell"].update(player_tokens=[6]))
    v3(on,"stale admission","v3 admission hash mismatch",edit=lambda m,*_:m.update(admission_sha256="0"*64))
    v3(on,"different private source","wrong v3 admission identity or phase",
       edit=lambda m,a,*_:a.update(source_current_sha256="0"*64))
    v3(on,"wrong binary authorization","v3 fixture/rubric/operator grant/executable bytes differ",
       edit=lambda m,a,*_:a.update(evaluator_sha256="0"*64))
    v3(on,"redirected private rubric","v3 requires a regular canonical sidecar",symlink=True)
    v3(on,"scope alias","experiment output already exists",preexisting=True)
    v3(on,"bad private geometry digest","v3 geometry predecessor hash mismatch",phase="opening",
       edit=lambda m,a,*_:a.update(geometry_proof_sha256="0"*64))
    v3(on,"crossed private geometry is not this layout","v3 predecessor is not exact zero-dispatch geometry",
       phase="opening",edit=change_geo)
    v3(on,"lossless long-number and escaped-string predecessor reaches only software-off fence",
       SOFTWARE_OFF,phase="opening",geometry_wire=lexical_geometry("good"))
    v3(on,"exact inner pin rejects one lexical byte despite valid outer sidecar pin",
       "v3 predecessor is not exact zero-dispatch geometry",phase="opening",geometry_wire=lexical_geometry("tamper"))
    v3(on,"typed predecessor refuses duplicate escaped setup key despite valid outer sidecar pin",
       "duplicate field `setup`",phase="opening",geometry_wire=lexical_geometry("duplicate"))
    v3(on,"TTL geometry-only is inadmissible","only TTL requires its own independently approved 1/1 opening pair",
       phase="ttl")
    v3(on,"TTL preserves long original geometry lexemes without admitting any opening pair",
       "only TTL requires its own independently approved 1/1 opening pair",
       phase="ttl",geometry_wire=lexical_geometry("good"))
    v3(on,"TTL refuses one changed original geometry byte despite exact outer digest",
       "v3 predecessor is not exact zero-dispatch geometry",
       phase="ttl",geometry_wire=lexical_geometry("tamper"))
    for n,e in (
        ("absent-pair",NO_FILE),("bad-pair-hash","TTL paired opening predecessor bytes differ"),
        ("wrong-opening-rubric","TTL operator grant or independently verified opening pair has a different identity"),
        ("wrong-geometry-rubric","TTL operator grant or independently verified opening pair has a different identity"),
        ("same-phase-rubric","TTL operator grant or independently verified opening pair has a different identity"),
        ("ttl-reuses-geometry-rubric","TTL operator grant or independently verified opening pair has a different identity"),
        ("ttl-reuses-opening-rubric","TTL operator grant or independently verified opening pair has a different identity"),
        ("wrong-independent-closure","TTL operator grant or independently verified opening pair has a different identity"),
        ("old-v1-proof","TTL operator grant or independently verified opening pair has a different identity"),
        ("old-v1-grant","TTL operator grant or independently verified opening pair has a different identity"),
        ("wrong-operator-grant","TTL operator grant or independently verified opening pair has a different identity"),
        ("bad-pair-parent-id","TTL operator grant or independently verified opening pair has a different identity"),
        ("bad-pair-alias","TTL predecessor must be two distinct crossed opening witnesses"),
        ("not-two-1-1","TTL predecessor is not the two exact separately granted 1/1 openings"),
    ):
        v3(on,n,e,phase="ttl",edit=ttl_bad_case(n))
    print("Compiled CLI boundaries only. No checkpoints, GPU, geometry result, accepted opening, or TTL authorization.")


if __name__ == "__main__":
    main(*sys.argv[1:])
