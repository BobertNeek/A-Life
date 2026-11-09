#!/usr/bin/env python3
"""Finite symbolic/public-source check. No Rust, subprocess or world calls."""
import hashlib,json,pathlib,re
ROOT=pathlib.Path(__file__).resolve().parent
NAMES=['Locomotion','Orientation','Manipulation','Vocal','Posture']
def digest(b):return hashlib.sha256(b).hexdigest()
def parse_arms(text,prefix):
    rows=[x.strip() for x in text.strip().splitlines()]
    expected=[f'{prefix}::{name} => {i},' for i,name in enumerate(NAMES)]+[f'{prefix}::SpeciesSpecific(id) => 0x100 + u16::from(id),']
    if rows!=expected:raise ValueError('MAPPING_SOURCE_MISMATCH')
    return {name:i for i,name in enumerate(NAMES)},0x100

def witness(core,source):
    declaration=re.search(r'pub enum MotorChannel \{(.*?)\n\}',core,re.S)
    if not declaration or [x.strip() for x in declaration[1].strip().splitlines()]!=[x+',' for x in NAMES]+['SpeciesSpecific(u8),']:
        raise ValueError('ENUM_SOURCE_MISMATCH')
    old=re.search(r'pub\(crate\) fn canonical_key\(self\) -> u16 \{\s*match self \{(.*?)\n        \}',core,re.S)
    new=re.search(r'(?m)^fn recovery_reflex_channel_key\(channel: MotorChannel\) -> u16 \{\s*match channel \{(.*?)\n    \}',source,re.S)
    if not old or not new:raise ValueError('HELPER_SIGNATURE_MISMATCH')
    a,abase=parse_arms(old[1],'Self');b,bbase=parse_arms(new[1],'MotorChannel')
    # Every public enum value: five unit variants and all 256 u8 IDs. This
    # interprets only the above allowlisted constant/from expressions in Python.
    left=[a[n] for n in NAMES]+[abase+i for i in range(256)]
    right=[b[n] for n in NAMES]+[bbase+i for i in range(256)]
    if left!=right or len(set(right))!=261 or not all(0<=n<=65535 for n in right):raise ValueError('FINITE_MAPPING_MISMATCH')
    if source.count('recovery_reflex_channel_key(c.channel)')!=2 or 'c.channel.canonical_key()' in source:raise ValueError('CALL_SITE_MISMATCH')
    return {'schema':'m2-reflex-channel-key-finite-witness-v2','status':'passed','public_enum_values':261,'unit_variants':5,'species_specific_ids':256,'mapping_exact':True,'injective':True,'u16_range_preserved':True,'sort_and_multiset_equality_preserved':True,'core_api_expansions':0,'casts_added':0,'rust_invocations':0,'native_fixtures':0,'world_calls':0}

def load_inputs():
    meta=json.loads((ROOT/'MANIFEST.json').read_text())
    core=(ROOT/'pinned-core-motor-channel.txt').read_bytes()
    if meta['revised_patch']['path']!='../patches/fbf-to-reflex.patch':raise ValueError('PATCH_PATH_MISMATCH')
    data=(ROOT.parent/'patches/fbf-to-reflex.patch').read_bytes()
    if len(data)!=meta['revised_patch']['bytes'] or digest(data)!=meta['revised_patch']['sha256']:raise ValueError('PATCH_HASH_MISMATCH')
    path=meta['changed_source']['path'];header=f'diff --git a/{path} b/{path}\n'.encode()
    if data.count(header)!=1:raise ValueError('PATCH_SECTION_MISMATCH')
    section=data.split(header,1)[1].split(b'diff --git ',1)[0]
    lines=section.splitlines(keepends=True)
    if lines[:3]!=[b'new file mode 100644\n',b'--- /dev/null\n',('+++ b/'+path+'\n').encode()]:raise ValueError('PATCH_HEADER_MISMATCH')
    match=re.fullmatch(rb'@@ -0,0 \+1,(\d+) @@\n',lines[3])
    if not match or len(lines[4:])!=int(match[1]) or any(not line.startswith(b'+') for line in lines[4:]):raise ValueError('PATCH_BODY_MISMATCH')
    source=b''.join(line[1:] for line in lines[4:])
    if digest(core)!=meta['core_source']['public_excerpt_sha256'] or digest(source)!=meta['changed_source']['sha256'] or len(source)!=meta['changed_source']['bytes']:raise ValueError('SOURCE_HASH_MISMATCH')
    return core.decode(),source.decode()

def main():
    result=witness(*load_inputs())
    (ROOT/'MAPPING-CHECK.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
if __name__=='__main__':main()
