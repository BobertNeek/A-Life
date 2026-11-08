"""Offline scalar/source-only analysis. No native execution or simulation."""
import sys,json,hashlib, pathlib,collections
sys.dont_write_bytecode=True
ROOT=pathlib.Path('/workspace/scratch/ee5e08a3ca9b'); OUT=pathlib.Path(__file__).resolve().parent
P=ROOT/'world-swap-completed-public-v5'; CONTRACT=ROOT/'world-swap-matched-23xx-active-v5/report_contract.py'
# Execute only the inspected pure-Python schema/receipt validator, never a launcher.
ns={};exec(compile(CONTRACT.read_text(),str(CONTRACT),'exec'),ns)
def load(p): return json.loads(p.read_bytes(),object_pairs_hook=ns['pairs'])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
manifest=load(P/'PUBLIC-COPY-MANIFEST.json')
assert all(sha(P/f)==h for f,h in manifest.items())
for line in (P/'SHA256SUMS').read_text().splitlines():
 h,f=line.split();assert sha(P/f)==h
rows=load(P/'rows.json');summary=load(P/'summary.json');bindings=load(P/'source-bindings.json');run=load(P/'AS-RUN.json');checks=load(P/'checks.json')
assert set(rows)=={'cases','incomplete_prefixes'} and rows['incomplete_prefixes']==[]
assert set(summary)=={'cases','complete_pairs','incomplete_prefixes'} and summary['incomplete_prefixes']==[]
assert set(bindings)=={'old_world','new_world'}
for b in bindings.values():ns['validate'](b,ns['SOURCE'])
assert bindings==run['source_bindings']
assert checks['terminal_gate']=='complete' and checks['case_sets']['validated']==[a['case_id'] for a in rows['cases']]
FAMILIES=['Idle','Rest','Inspect','Approach','Avoid','Contact','Ingest','Other']
def enriched(c,seqs,first):
 c=dict(c);c['family_name']=FAMILIES[c['family']];seq=c['source_sequence'];c['source_is_first_harmful_ingestion']=bool(c['target']=='h' and c['source_exact_ingest'] and seq==first)
 c['source_classification']=('exact_harmful_ingestion' if c['source_is_first_harmful_ingestion'] else 'exact_safe_ingestion' if c['target']=='s' and c['source_exact_ingest'] else 'no_best_family_source' if seq is None else 'non_exact_ingestion_best_family_source')
 if seq in seqs:
  r,d=seqs[seq];c['source_decision_scalar_context']={'primary_tick':r['primary_tick'],'sequence':seq,'physical_meals':d['physical_meals'],'experienced_valence':d['experienced_valence'],'aversive_harm':d['aversive_harm'],'channels':d['channels']}
 return c
cases=[]
for a in rows['cases']:
 ns['validate'](a,ns['CASE']);assert a['source']==bindings[a['world']]
 assert ns['validate_rows'](a['rows'])==a['stop'];assert ns['metrics'](a)==next(s['metrics'] for s in summary['cases'] if s['case_id']==a['case_id'])
 ds=[(r,d) for r in a['rows'] for d in r['decisions']];seqs={d['sequence']:(r,d) for r,d in ds}
 meals=[(r,d) for r,d in ds if d['physical_meals'][0]];assert len(meals)==1
 r0,d0=meals[0];first=d0['sequence'];acq=[];selected_h=[];held=[]
 for r,d in ds:
  if d['sequence']<=first:continue
  if next(o for o in d['owners_before'] if o['target']=='h')['owner']=='self':held.append(r['primary_tick'])
  for ch in d['channels']:
   if ch['slot']==2 and ch['target']=='h':selected_h.append({'primary_tick':r['primary_tick'],'sequence':d['sequence'],'channel':ch})
   if any(t['kind']=='acquired' and t['target']=='h' for t in ch['transfers']):
    chosen=d['candidates'][ch['candidate']]
    acq.append({'primary_tick':r['primary_tick'],'world_tick_before':r['world_tick_before'],'sequence':d['sequence'],'channel':ch,'owners_before':d['owners_before'],'owners_after':d['owners_after'],'selected_candidate':enriched(chosen,seqs,first),'all_food_candidates':[enriched(c,seqs,first) for c in d['candidates'] if c['target'] in ('h','s')],'physical_meals':d['physical_meals']})
 cases.append({'case_id':a['case_id'],'first_harmful_meal':{'primary_tick':r0['primary_tick'],'sequence':first,'world_tick_before':r0['world_tick_before'],'aversive_harm':d0['aversive_harm'],'experienced_valence':d0['experienced_valence'],'physical_meals':d0['physical_meals'],'channels':d0['channels']},'later_h_acquisitions':acq,'later_h_manipulation_selections':selected_h,'later_h_held_before_decision_ticks':held,'counts':{'h_reacquisitions':len(acq),'grab_reacquisitions':sum(z['channel']['primitive']==211 for z in acq),'eat_preparation_reacquisitions':sum(z['channel']['primitive']==210 for z in acq),'later_h_eat_selections':sum(z['channel']['primitive']==210 for z in selected_h),'later_h_meals':0,'held_before_decisions':len(held)}})
source_rel=['crates/alife_training/examples/support/world_swap_receipt_projection.rs','crates/alife_training/src/cpu/host/swap_audit.rs','crates/alife_core/src/perception.rs','crates/alife_core/src/memory.rs','crates/alife_world/src/headless.rs']
sources={str(CONTRACT):sha(CONTRACT)}
for world in ['old','new']:
 for rel in source_rel:
  p=ROOT/f'world-swap-matched-source-{world}-v5'/rel;sources[str(p)]=sha(p)
result={'schema':'offline_h_reacquisition_analysis_v1','input_public_manifest_sha256':sha(P/'PUBLIC-COPY-MANIFEST.json'),'verified_public_files':manifest,'source_sha256':sources,'validation':{'public_hashes':True,'closed_case_schemas':True,'row_receipt_semantics':True,'source_bindings_equal':True,'recomputed_metrics_equal':True,'original_native_bytes_independently_revalidated_here':False,'original_native_byte_limitation':'Original native summary bytes are not in this permitted copy; checks and AS-RUN record prior verification. Re-serialized public cases are not claimed byte-identical.'},'cases':cases}
(OUT/'analysis.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
lines=['# Harmful-food re-acquisitions after the first harmful meal','','Offline descriptive analysis only; no new life, exposure, replay, native execution, or source change.','', '## Results','']
for c in cases:
 lines.append(f"- {c['case_id']}: first harmful meal tick/sequence {c['first_harmful_meal']['primary_tick']}/{c['first_harmful_meal']['sequence']}; {c['counts']['h_reacquisitions']} later actual H acquisitions, ticks " + ', '.join(str(z['primary_tick']) for z in c['later_h_acquisitions']) + f". Counts: {c['counts']}.")
lines += ['', '## Exact recall fields at each H acquisition','', 'All source and confidence numbers below are copied without rounding. Full candidates for H and S, source-decision context, ownership and receipts are in analysis.json. Each row contrasts the chosen candidate with concurrent H/EAT recall. Source sequence refers only to best FAMILY source; target-source identities and target valence are not exported. Family valence is an aggregate family-value channel, not necessarily the valence of the single best source.','']
for c in cases:
 lines += ['### '+c['case_id'],'']
 for z in c['later_h_acquisitions']:
  ch=z['selected_candidate'];eat=next((x for x in z['all_food_candidates'] if x['target']=='h' and x['primitive']==210),None)
  keys=['target_sources','target_confidence','family_sources','family_confidence','family_valence','source_sequence','source_exact_ingest','source_classification']
  lines += [f"- Tick/sequence {z['primary_tick']}/{z['sequence']}: primitive {ch['primitive']}, family {ch['family']} ({ch['family_name']}), event {z['channel']['event']}, phase {z['channel']['phase']}.", '  - Chosen: '+json.dumps({k:ch[k] for k in keys}), '  - Concurrent H/EAT: '+(json.dumps({k:eat[k] for k in keys}) if eat else 'not present')]
lines += ['', '## Interpretation and limits','', '- Acquisition requires an actual receipt transfer of H from nobody to self. An EAT selection, failed attempt, or continuing ownership alone is not counted as a new acquisition. EAT preparation acquires without consuming. All 32 reacquisitions have event acquired, not atomic_swap; the reverse S-to-H swap in the new seed-2 case occurs before its first harmful meal.', '- 31 reacquisitions use GRAB/Contact, and one uses EAT/Ingest preparation. Contact best-family sources are not exact ingestion evidence; their source-decision receipts are included, without pretending that a multichannel decision uniquely identifies an unexported memory episode. A non-null source sequence alone does not prove harmful ingestion.', '- The exact-ingestion flag binds the best family source to EAT + Ingest + Consumed + the same tracked target. Coupled with the validated prior physical meal and sequence it identifies the harmful ingestion specifically. An exact safe-food ingestion recall is not harmful-food recall.', '- Observed negative H/Ingest recall alongside choosing H/Contact motivates inspecting cross-family use of outcome evidence. It does not establish missing retrieval, learned avoidance, causal credit assignment, or why the action was chosen. A selected EAT preparation with negative exact harm recall is likewise descriptive, not proof of a brain defect.', '- Only two seeds and a bundled world change. Additional reacquisitions are not by themselves worse behavior: opportunities and release/holding duration differ. These data contain no candidate action probabilities, target-source identities, neural activations or ablations. No hidden-state or causal-learning inference is supported.', '- All public-file manifest and SHA256SUMS hashes passed, all closed CASE schemas and row receipt semantics passed, source bindings match AS-RUN, and metrics exactly recompute. Original native summary bytes are outside this permitted copy; their previous verification is recorded by checks/AS-RUN, not independently repeated here.', '- The requested active-v5/receipt_projection.rs path does not exist. The identically hashed projection file in both frozen source repos is used instead: crates/alife_training/examples/support/world_swap_receipt_projection.rs (e1a4a48ad800c3290ac8e046b427d47099ae65812bfc4ed0d75a82908330061c), especially lines 245–261. report_contract.py labels itself reconstructed, not byte-equivalent to an earlier inaccessible original. No equivalence to that original is claimed.', '', '## Reproduction','', 'Run python -B '+str(OUT/'analyze.py')+'. This only reads the approved public/scalar/source inputs and writes this analysis directory. Exact input paths and hashes are recorded in analysis.json.']
(OUT/'NOTES.md').write_text('\n'.join(lines)+'\n')
for c in cases:
 print(c['case_id'],c['counts'])
 print('H EAT exact harm at grabs',[(z['primary_tick'],next((x['source_is_first_harmful_ingestion'] for x in z['all_food_candidates'] if x['target']=='h' and x['primitive']==210),None)) for z in c['later_h_acquisitions']])
