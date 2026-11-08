import json,hashlib,pathlib
source=pathlib.Path('../biological-drive-audit-v1')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,obj):pathlib.Path(name).write_text(json.dumps(obj,indent=2,sort_keys=True)+'\n')
files=['world-'+x+'-v2.json' for x in ['baseline','candidate','recalibrated','geometry']]+['world-distance-v1.json']
channels=['Locomotion','Orientation','Manipulation','Vocal','Posture','SpeciesSpecific (id unknown)'];arms=[]
for filename in files:
 p=json.loads((source/filename).read_text());scenarios=[]
 for s in p['results']:
  ticks=[]
  for ri,r in enumerate(s['rows']):
   candidates={c['candidate_index']:c for c in r['frame']['base']['candidates']};o=r['outcome'];joint=o.get('joint') or {};outcomes=joint.get('channel_outcomes',[]);selections=[]
   for slot,index in enumerate(r['action']['motor_candidates']):
    c=candidates.get(index);phys=[x['physical'] for x in outcomes if x['channel']==channels[slot]]
    selections.append(dict(channel_slot=slot,channel=channels[slot],selected_candidate_index=index,selected_candidate=None if c is None else {k:c[k] for k in ['action_id','family','kind','target']},resolved_primitive=None,physical_success=None,channel_physical_records=phys,consumed_contact_observed=any(x.get('contact')=='Consumed' for x in phys)))
   ticks.append(dict(raw_row_index=ri,outcome_tick=o['outcome_tick'],sequence_id=o['sequence_id'],selected_channels=selections,aggregate_success=o['success'],aggregate_physical=o['physical'],joint_channel_outcomes=outcomes,joint_channel_observations=joint.get('channel_observations'),joint_execution=joint.get('execution'),persistent_consumed_after_tick=None,consumed_contact_observed=any(x['physical'].get('contact')=='Consumed' for x in outcomes)))
  scenarios.append(dict(name=s['name'],final_consumed=s['consumed'],final_position=s['final_position'],ticks=ticks))
 arms.append(dict(mode=p['mode'],raw_file=filename,raw_sha256=sha(source/filename),foundation_digest=p['foundation_digest'],phenotype_hash=p['phenotype_hash'],scenarios=scenarios))
write('actions-v2.json',dict(schema_version='alife.drive_action_projection.v2',arms=arms))
write('schema-v2.json',{'schema_version':'alife.drive_action_projection.v2','scope':'Projection of existing disposable controlled records only. No native execution or learned checkpoint.','structure':'arms[] -> scenarios[] -> ticks[] -> selected_channels[6]','field_rules':{'raw_row_index':'Zero-based index in original scenario rows.','selected_candidate':'Null for unselected slot; otherwise exact candidate action_id/family/kind/target copied using selected candidate_index.','channel':'Slots0–4 are fixed native channel names; slot5 species ID not serialized, marked unknown.','resolved_primitive':'Always null/unknown: raw did not serialize resolved Move/Interact or other motor primitive. Do not infer from kind or ID.','physical_success':'Always null/unknown: raw has aggregate success only, not channel success.','channel_physical_records':'Exact joint.channel_outcomes physical records matched to named channel. Empty list means none recorded, not failure.','aggregate_success':'Exact raw outcome.success. Does not establish every selected channel succeeded.','consumed_contact_observed':'Exact comparison of recorded contact strings to Consumed; event flag, not persistent entity state.','persistent_consumed_after_tick':'Always null/unknown: persistent per-tick consumed state was not logged.','final_consumed':'Exact final scene consumed Boolean.','joint_channel_observations':'Exact recorded executed/intensity/displacement observations, not inferred selection or primitive.'}})
bindings={f:sha(source/f) for f in files+['world-summary-v1.json','drive-matrix-v1.json','crates/alife_training/examples/probe_drive_world.rs','crates/alife_training/examples/support/drive_balance_candidate.rs','crates/alife_core/src/motor.rs']}
write('source-bindings-v2.json',{'baseline_commit':'2da9dd323ad6f074556a851174a2edbc9e34e216','baseline_tree':'e402bcb5d5fc034ac0244e78d49fa1a0da25ede3','source_and_record_sha256':bindings,'frozen_source_patch_sha256':sha(pathlib.Path('../biological-drive-audit-v1-source.patch')),'frozen_evidence_archive_sha256':sha(pathlib.Path('../biological-drive-audit-v1-evidence.tar.gz'))})
assert len(arms)==5 and all(len(a['scenarios'])==17 and all(len(s['ticks'])==8 for s in a['scenarios']) for a in arms)
print('Validated 5 arms, 85 scenarios, 680 ticks, 4080 channel slots; no primitive inferred.')
