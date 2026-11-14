#!/usr/bin/env python3
"""LOCAL read-only reconstruction of a recorded category target-recall shortlist."""
import argparse
import json
import math
from pathlib import Path
import struct


def f32(x):
    return struct.unpack('<f',struct.pack('<f',x))[0]


def frombits(x):
    return struct.unpack('<f',struct.pack('<I',x))[0]


def rounded(x):
    return math.floor(x+0.5) if x>=0 else math.ceil(x-0.5)


def cosine(a,b,lo,hi):
    dot=left=right=0.
    for i in range(lo,hi):
        dot=f32(dot+f32(a[i]*b[i]))
        left=f32(left+f32(a[i]*a[i]))
        right=f32(right+f32(b[i]*b[i]))
    if left==right==0:return 1.
    if left==0 or right==0:return 0.
    return max(-1.,min(1.,f32(dot/f32(f32(math.sqrt(left))*f32(math.sqrt(right))))))


def category(a):
    return [rounded(f32(a[57+i]*4.)) for i in list(range(6,18))+[21]]


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('run',type=Path)
    p.add_argument('--interval',type=int,default=102)
    args=p.parse_args()
    stem=f'STEP-{args.interval:03}'
    row=json.loads((args.run/f'{stem}.rows.json').read_text())[0]
    pre=json.loads((args.run/f'{stem}-PRE.cp.json').read_text())
    query=row['patch']['decision']['episodic_key']['query']
    receipt=json.loads((args.run/f'{stem}-0-RECALL.json').read_text())
    index=row['action']['representative']
    context=receipt['context']['candidates'][index]
    evidence=receipt['receipt']['candidates'][index]
    features=list(map(f32,query['features']))
    records=pre['residents'][0]['memory']['records']
    assert not any(r['tracked_object_id_raw']==query['tracked_object_id'] for r in records), 'this narrow checker requires an actually new identity'
    candidates=[]
    for record in records:
        source=list(map(frombits,record['query_feature_bits']))
        if not record['tracked_object_id_raw'] or category(features)!=category(source):continue
        if any(abs(features[57+i]-source[57+i])>0.2 for i in list(range(6,18))+[21]):continue
        if any(cosine(features,source,57+i,57+i+3)<0.9 for i in range(6,18,3)):continue
        score=f32(f32(f32(f32(.45)*cosine(features,source,0,40))+f32(f32(.50)*cosine(features,source,57,81)))+f32(f32(.05)*cosine(features,source,81,83)))
        if score < .55:continue
        candidates.append({'memory_id':record['memory_id_raw'],'source_tick':record['last_tick_raw'],
          'family_raw':record['family_raw'],'outcome_contact':record.get('outcome_contact'),
          'hunger_delta':frombits(record['target_latent_bits'][0]),'energy_delta':frombits(record['family_value_bits'][3]),'similarity':score})
    candidates.sort(key=lambda r:(-r['similarity'],r['memory_id']))
    assert len(candidates)==evidence['target_eligible']
    top=candidates[:4]
    total=0.
    for r in top:total=f32(total+r['similarity'])
    hunger=0.
    for r in top:hunger=f32(hunger+f32(r['hunger_delta']*f32(r['similarity']/total)))
    assert abs(hunger-context['target_latent'][0])<2e-8,(hunger,context['target_latent'][0])
    assert top[0]['memory_id']==context['best_target_source']
    result={'passed':True,'decision_tick':query['tick'],'selected_family':query['action_family'],
      'candidate_index':index,'new_identity_without_exact_prior':True,
      'native_target_eligible':evidence['target_eligible'],'native_target_matches':evidence['target_matches'],
      'ranked_category_sources':candidates,'selected_ids':[r['memory_id']for r in top],
      'reconstructed_hunger_delta':hunger,'native_hunger_delta':context['target_latent'][0],
      'native_confidence':context['target_confidence'],'source_selection_is_similarity_not_terminal_value':True}
    with(args.run/'SELECTED-SOURCE-AGGREGATES.json').open('x')as f:json.dump(result,f,indent=2)
    print(json.dumps(result))


if __name__=='__main__':main()
