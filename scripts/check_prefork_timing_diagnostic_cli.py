"""Published-stock binary refusal checks. There is no real Current or custody."""
from pathlib import Path
from tempfile import TemporaryDirectory
import json
import subprocess
import sys
import time

off,on=(str(Path(x).resolve(strict=True)) for x in sys.argv[1:])
with TemporaryDirectory(prefix='artificial-prefork-denials-') as item:
    root=Path(item).resolve()
    assert root.stat().st_mode & 0o777 == 0o700
    good=root/'artificial-unit'
    good.mkdir(mode=0o700)
    current=good/'Current.alife-checkpoint'
    target=good/'native-output'
    manifest=good/'experiment.json'

    def refus(exe,m,input_path=current,out=target,*,fixed=True):
        manifest.write_bytes(json.dumps(m,sort_keys=True,separators=(',',':')).encode()+b'\n')
        paths=tuple(sorted(f.relative_to(root).as_posix() for f in root.rglob('*')))
        env={'PATH':'/usr/bin:/bin','LANG':'C'}
        if fixed:env['ALIFE_EXPLICIT_FIXED_HOST_PRESSURE_NS']='0'
        p=subprocess.run([exe,'--experiment-manifest',str(root),str(input_path),str(out),str(manifest)],
            cwd=root,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=10)
        assert p.returncode==1 and not p.stdout and p.stderr and not target.exists(),(p.returncode,p.stderr)
        assert tuple(sorted(f.relative_to(root).as_posix() for f in root.rglob('*')))==paths
        return p.stderr.decode('utf8')

    def contains(stage,e):
        assert stage in e,(stage,e)

    deadline=int(time.time())+200  # CLI request validator only, not host admission.
    ordinary=dict(schema=1,run_id='artificial-wire',owner=77,scenario='retained_opening',
        source_current_sha256='a'*64,intervention={'kind':'control'},evaluation_nonce=17,
        max_decisions=2,max_world_ticks=2,deadline_unix_seconds=deadline,max_output_bytes=268435456)
    stock=dict(ordinary,scenario='published_card_case',card_case='r12-word-empty-silence',
        max_decisions=256,max_world_ticks=256)
    previous=[
        ordinary,{'schema':8},{'schema':10},
        *[{'schema':81,'settings':stock,'mode':mode,'window_start_tick':41,
         'operator_grant_sha256':'b'*64} for mode in ('reference','capture')]
    ]
    for wire in previous:
        a,b=(refus(x,wire) for x in (off,on))
        assert a==b and 'prefork timing diagnostic' not in a

    valid_shape=dict(schema=82,settings=stock,custody=dict(fence_fd=77,audit_socket_fd=78,residual_memfd=79))
    contains('prefork timing diagnostic feature unavailable',refus(off,valid_shape))
    contains('No such file',refus(on,valid_shape))
    # Only an inert filesystem sentinel for later refusal phases; it never
    # contains a valid source header, genotype, or native life.
    current.write_bytes(b'not a checkpoint\n')
    wrong=good/'unrelated-source'
    wrong.write_bytes(b'not a checkpoint\n')
    contains('noncanonical new role',refus(on,valid_shape,wrong))
    contains('unknown field',refus(on,dict(valid_shape,unsupported=True)))
    contains('typed inherited custody',refus(on,dict(valid_shape,
        custody=dict(fence_fd=77,audit_socket_fd=77,residual_memfd=79))))
    contains('stock Control 256/256',refus(on,dict(valid_shape,
        settings=dict(stock,max_world_ticks=8))))
    contains('stock Control 256/256',refus(on,valid_shape,fixed=False))
    # Even before any ticket/profile contents, the exact role directory
    # has to be canonical and owner0700. Merely removing write access is not enough.
    good.chmod(0o750)
    contains('owner0700 cohort and role rooms',refus(on,valid_shape))
    good.chmod(0o700)
    # There is no root ticket, protected parent, process fixture, or usable FD.
    contains('missing inherited supervisor descriptor',refus(on,valid_shape))
    assert current.read_bytes()==wrong.read_bytes()==b'not a checkpoint\n'
print('prefork_asset_free_existing_cli_equivalence_and_new_refusals=PASS')
