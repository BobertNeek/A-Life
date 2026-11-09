#!/usr/bin/env python3
import argparse, hashlib, json, pathlib, re, sys
R=pathlib.Path(__file__).resolve().parent
PLAN=R.name
PREFIX="docs/experiments/"+PLAN+"/"
MAIN_TREE="9140d525fe1af16fed6539c3178a1a45e1fb04d0"
def require(x):
    if not x: raise ValueError("INVENTORY")
def blob(b):
    return hashlib.sha1(b"blob "+str(len(b)).encode()+b"\0"+b).hexdigest()
def expected(phase,prepared):
    seal=json.loads((R/"seal.json").read_text())
    reuse={x["path"]:x for x in json.loads((R/"REUSE-MANIFEST.json").read_text())["files"]}
    out={}
    names=[x["path"] for x in seal["files"]]
    require(len(names)==len(set(names)))
    records={x["path"]:x for x in seal["files"]}
    for name in names+["seal.json","workflow.yml","ACTIVATE.template.json"]:
        p=pathlib.Path(reuse[name]["source_artifact"]) if name in reuse else R/name
        b=p.read_bytes()
        if name in records:
            row=records[name]
            require(len(b)==row["bytes"] and hashlib.sha256(b).hexdigest()==row["sha256"])
        out[PREFIX+name]=("100644","blob",blob(b))
    out[".github/workflows/"+PLAN+".yml"]=out[PREFIX+"workflow.yml"]
    if phase=="activated":
        require(isinstance(prepared,str) and re.fullmatch("[0-9a-f]{40}",prepared))
        marker=json.loads((R/"ACTIVATE.template.json").read_text())
        marker["prepared_head"]=prepared
        # Publisher must use these exact canonical marker bytes.
        b=(json.dumps(marker,indent=2,sort_keys=True)+"\n").encode()
        out[PREFIX+"ACTIVATE.json"]=("100644","blob",blob(b))
    require(PREFIX+"ARCHIVE-MANIFEST.json" not in out)
    require(PREFIX+"source-only-m2-play-hosted-qualification-v3-update.tar.gz" not in out)
    return out
def leaves(payload,tree):
    require(payload.get("truncated") is False and payload.get("sha")==tree)
    require(isinstance(payload.get("tree"),list))
    seen=set();out={}
    for row in payload["tree"]:
        p=row.get("path")
        require(isinstance(p,str) and re.fullmatch(r"[A-Za-z0-9_.\-/]+",p))
        require(not p.startswith("/") and ".." not in pathlib.PurePosixPath(p).parts and p not in seen)
        seen.add(p)
        require(row.get("type") in ("tree","blob","commit"))
        require(isinstance(row.get("sha"),str) and re.fullmatch("[0-9a-f]{40}",row["sha"]))
        if row["type"]!="tree":
            out[p]=(row.get("mode"),row["type"],row["sha"])
    return out
def verify(candidate,main,tree,phase,prepared=None):
    require(phase in ("prepared","activated"))
    require(isinstance(tree,str) and re.fullmatch("[0-9a-f]{40}",tree))
    old=leaves(main,MAIN_TREE)
    actual=leaves(candidate,tree)
    extra=expected(phase,prepared)
    require(not (set(old)&set(extra)))
    require(actual==dict(old,**extra))
    rows=json.loads((R/"WORKFLOW-PRESERVATION.json").read_text())["files"]
    require(len(rows)==15)
    inherited={x["path"]:("100644","blob",x["git_blob_sha1"]) for x in rows}
    require(all(old.get(p)==v for p,v in inherited.items()))
    workflows={p:v for p,v in actual.items() if p.startswith(".github/workflows/")}
    require(workflows==dict(inherited,**{
        ".github/workflows/"+PLAN+".yml":extra[".github/workflows/"+PLAN+".yml"]}))
    return len(extra)
def regress():
    rows=json.loads((R/"WORKFLOW-PRESERVATION.json").read_text())["files"]
    old={x["path"]:("100644","blob",x["git_blob_sha1"]) for x in rows}
    def tree(d,s):
        return {"sha":s,"truncated":False,"tree":[
            {"path":p,"mode":v[0],"type":v[1],"sha":v[2]} for p,v in sorted(d.items())]}
    main=tree(old,MAIN_TREE)
    for phase in ("prepared","activated"):
        prepared="1"*40 if phase=="activated" else None
        good=dict(old,**expected(phase,prepared))
        require(PREFIX+"qualified-original-overlay.tar.gz" in good)
        verify(tree(good,"2"*40),main,"2"*40,phase,prepared)
        extra=dict(good)
        extra[PREFIX+"ARCHIVE-MANIFEST.json"]=("100644","blob","3"*40)
        missing=dict(good);missing.pop(PREFIX+"run_hosted.py")
        wrong=dict(good);wrong[PREFIX+"run_hosted.py"]=("100644","blob","4"*40)
        archive_extra=dict(good)
        archive_extra[PREFIX+"source-only-m2-play-hosted-qualification-v3-update.tar.gz"]=("100644","blob","5"*40)
        for bad in (extra,archive_extra,missing,wrong):
            try: verify(tree(bad,"2"*40),main,"2"*40,phase,prepared)
            except ValueError: pass
            else: raise AssertionError("Accepted invalid remote inventory")
        truncated=tree(good,"2"*40);truncated["truncated"]=True
        try: verify(truncated,main,"2"*40,phase,prepared)
        except ValueError: pass
        else: raise AssertionError("Accepted truncated tree")
if __name__=="__main__":
    try:
        require(__debug__)
        p=argparse.ArgumentParser()
        p.add_argument("--self-test",action="store_true")
        p.add_argument("--tree")
        p.add_argument("--main-tree")
        p.add_argument("--expected-tree")
        p.add_argument("--phase",choices=("prepared","activated"))
        p.add_argument("--prepared-head")
        a=p.parse_args()
        if a.self_test:
            regress();count=None
        else:
            count=verify(json.loads(pathlib.Path(a.tree).read_text()),
                         json.loads(pathlib.Path(a.main_tree).read_text()),
                         a.expected_tree,a.phase,a.prepared_head)
        print(json.dumps({"ok":True,"code":"OK","expected_added_leaf_count":count}))
    except BaseException:
        print('{"ok":false,"code":"INVENTORY"}')
        sys.exit(1)
