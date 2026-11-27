"""Isolated checks on real compiled binaries. Never provides Current or opens Vulkan."""
import hashlib,importlib.util,json,os,subprocess,sys,time
from pathlib import Path
from tempfile import TemporaryDirectory
OFF,ON,helper,old_rust = (Path(x).resolve(strict=True) for x in sys.argv[1:])
q = importlib.util.spec_from_file_location('exact_existing_v3_device_free_checks',helper)
mod=importlib.util.module_from_spec(q)
q.loader.exec_module(mod)
need=mod.require
need(mod.pins(OFF)!=mod.pins(ON),'feature binaries alias')
caps=lambda e:subprocess.run([e,'--capabilities'],env=mod.ENV,capture_output=True,check=True,timeout=8).stdout
need(caps(OFF)==caps(ON),'existing capability bytes changed')
for name,case in [('retained_opening',None),('published_card_case','r2-word-retreat-food')]:
    need(mod.v1(OFF,name,mod.SOFTWARE_OFF,case)==mod.v1(ON,name,mod.SOFTWARE_OFF,case),'existing shared scorecard CLI changed')
source=old_rust.read_text()
import re
contract=tuple(re.search(r'^const '+n+r': &str = "([^"]+)";$',source,re.M)[1] for n in ('ID','SOURCE','PLAN'))
for e in (OFF,ON):
    for l in ('a','b'):
        for p in ('geometry','opening'):
            mod.v3(e,'unchanged v3 / '+l+' / '+p,mod.NO_FILE,phase=p,layout=l,marker=False)
        mod.v3(e,'legacy TTL unchanged geometry prerequisite',
               'only TTL requires its own independently approved 1/1 opening pair',phase='ttl',layout=l)
        mod.v2(e,'unchanged ordinary schema5',mod.NO_FILE,contract,layout=l,words=(9,14),marker=False)
    mod.v3(e,'unchanged legacy opening forbids two words','v3 manifest phase, scope or genuine Player token vector refused',
           phase='opening',edit=lambda m,*_:m['cell'].update(player_tokens=[9,14]))
def calls(exe,manifest,name,expected,setup=lambda r:None):
    with TemporaryDirectory(prefix='qualia-wsp-v2-synthetic-no-native-') as d:
        root=Path(d).resolve(strict=True)
        setup(root)
        return mod.call(root,exe,manifest,name,expected)
scope='qualia.saved_life.wsp_approach_v2.v1'
m={'schema':11,'scope':scope,'inner_sha256':'a'*64,'admission_sha256':'b'*64}
calls(OFF,m,'actual schema11 compiled OFF','WSP Approach V2 feature is disabled')
calls(ON,{**m,'treatment_or_target_id':3},'actual schema11 rejects extra policy field','unknown field')
calls(ON,{**m,'scope':'not-a-project-scope'},'actual schema11 refuses untrusted scope','unexpected WSP Approach V2 envelope')
calls(ON,m,'actual schema11 reaches only missing local permission (no Current)','No such file or directory')
calls(ON,m,'actual schema11 rejects present malformed permission before Current','WSP Approach V2 exact local leaf mismatch',
      lambda r:(r/'wsp-approach-v2-admission.json').write_text('{}'))
# The detached grammar itself does not create an alternate route to mutate policy under old schema6.
mod.v3(ON,'legacy outer new grammar still lacks independent schema11 authority','wrong v3 admission identity or phase',
       phase='opening',edit=lambda m,*_:m.update(phase='wsp_approach_v2'))
print('shared v1/v3 boundaries, schema11 off/on guards and no-artifact fixture controls PASS')
