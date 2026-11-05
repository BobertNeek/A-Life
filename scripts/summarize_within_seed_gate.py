"""Score the frozen V27 gate LP on recorded V28 inputs. Never refit or install it."""
from pathlib import Path
import argparse
import ctypes,hashlib,json
import numpy as np
import zstandard as zstd
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--run', type=Path, required=True, help='Completed 16-decision native output')
parser.add_argument('--fit', type=Path, required=True, help='Previously frozen V27 separability-results.json')
parser.add_argument('--actor', type=Path, required=True, help='Original actor-Untouched.json.zst')
parser.add_argument('--preregistration', type=Path, required=True, help='Pre-capture plan with frozen fit and protected hashes')
parser.add_argument('--output-dir', type=Path, required=True, help='Directory without frozen-separator-results.json')
args = parser.parse_args()
ROOT, OUT, FIT, ACTOR = args.run, args.output_dir, args.fit, args.actor
OUT.mkdir(parents=True, exist_ok=True)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def rd(p):
 with zstd.ZstdDecompressor().stream_reader(open(p,'rb'))as f:return json.load(f)
plan=json.load(open(args.preregistration));assert sha(FIT)==plan['offline_fit_sha256']
fit=json.load(open(FIT));assert sha(ACTOR)==next(value for path,value in fit['input_sha256'].items() if Path(path).name == ACTOR.name)
source=rd(ACTOR);native=json.load(open(ROOT/'results.json'))
assert native['total_native_steps_and_scored_decisions']==16 and native['preparation_decisions']==0
assert native['native_or_shadow_optimizer_updates']==0
W=np.array(source['weights'],dtype=np.float32)[27648:28672].reshape(32,32)
# Local matrix only; no brain, host, trainer, checkpoint or model is constructed.
FROZEN=W.copy()
for gate,p in enumerate(fit['offline_minimum_linf_margin_probes']['0.1']):
 FROZEN[:,16+gate]=np.array(W[:,16+gate].astype(np.float64)+np.array(p['delta_coefficients_offline_only']),dtype=np.float32)
libm=ctypes.CDLL('libm.so.6');libm.tanhf.argtypes=[ctypes.c_float];libm.tanhf.restype=ctypes.c_float
def readout(weights,row):
 inp=np.array(row['final_features'][1856:1888],dtype=np.float32)
 off=np.array(row['replay_tick']['effective_weight_offsets'],dtype=np.float32)[27648:28672].reshape(32,32)
 outputs=[]
 for step in range(6):
  raw=np.zeros(32,dtype=np.float32)
  for lane in range(32):raw+=inp[lane]*(weights[lane]+off[lane])
  outputs.append(raw.copy());inp[18:]=[libm.tanhf(float(v))for v in raw[18:]]
 return np.array(outputs)
def decode(raw):
 tokens=[];steps=[]
 for i,r in enumerate(raw):
  token=sum(1<<bit for bit in range(8)if r[8+bit]>0)
  if r[16]>0 and token:tokens.append(token)
  steps.append(i)
  if r[17]>0:break
 return {'tokens':tokens,'visited_steps':steps,'gate_logits_all_six_steps':raw[:,16:18].tolist()}
def diff(a,b,p=''):
 if type(a)!=type(b):return[p]
 if isinstance(a,dict):
  keys=a.keys()|b.keys();return sum((diff(a.get(k),b.get(k),p+'/'+str(k))for k in sorted(keys)),[])
 if isinstance(a,list):
  if len(a)!=len(b):return[p]
  return sum((diff(x,y,p+'/'+str(i))for i,(x,y)in enumerate(zip(a,b))),[])
 return[]if a==b else[p]
records=[];groups={};raw_exact=[];all_input_hashes={str(FIT):sha(FIT),str(ACTOR):sha(ACTOR),str(ROOT/'results.json'):sha(ROOT/'results.json')}
for i in range(1,17):
 path=ROOT/f'native-decision-{i:02}.json.zst';all_input_hashes[str(path)]=sha(path)
 item=rd(path);r=item['record'];row=item['full_native_row'];a=readout(W,row);b=readout(FROZEN,row)
 saved=np.array(r['raw_speech_outputs_six_steps'],dtype=np.float32)
 assert np.array_equal(a.view('u4'),saved.view('u4')),'source arithmetic differs from captured native raw outputs'
 raw_exact.append(True)
 source_payload=decode(a);pred=decode(b)
 existing=r['conditional_native_payload'];assert source_payload['tokens']==([]if existing is None else existing['tokens'])
 label=r['label'];matching=r['actual_matching_declare'];positive=label=='supported_recent';negative=label=='unsupported_observed_prefix'
 if positive:
  assert matching and r['recent_matching_support']is not None
  e=r['recent_matching_support'];assert e['lesson']=='GetFood'and 0<r['tick']-e['tick']<=3
 if negative:
  assert matching and r['current_actual_event']is None and r['owned_before'][0]is None and r['owned_after'][0]is None
  assert all(e['lesson']!='GetFood'for e in r['prior_actual_events'])
 verdict=('preserved_valid_report'if pred['tokens']==[9,1]else'failed_valid_report')if positive else ('suppressed_unsupported_report'if pred['tokens']==[]else'failed_unsupported_report')if negative else'abstain_unlabeled_actual_state'
 entry={'number':i,'seed':r['seed'],'condition':r['condition'],'offset':r['offset'],'tick':r['tick'],'label':label,'actual_emitted':r['actual_emitted'],'prior_actual_events':r['prior_actual_events'],'current_actual_event':r['current_actual_event'],'recent_matching_support':r['recent_matching_support'],'latest_prior_event_matches':r['latest_prior_event_matches'],'owned_before':r['owned_before'],'owned_after':r['owned_after'],'actual_food_consumption':r['actual_food_consumption'],'prompted_cooldown_ready':r['prompted_cooldown_ready'],'unprompted_cooldown_ready':r['unprompted_cooldown_ready'],'first_two_heard_slots':r['first_two_heard_slots'],'native_conditional_payload':source_payload,'frozen_offline_conditional_payload':pred,'verdict':verdict,'safety':r['safety'],'native_full_row_sha256':r['native_full_row_sha256'],'new_global_optimizer_updates':0}
 records.append(entry);groups[(r['seed'],r['condition'],r['offset'])]=(entry,item)
pairs=[]
for seed in plan['seeds']:
 for offset in range(4):
  a,ar=groups[(seed,'near_opportunity',offset)];b,br=groups[(seed,'far_opportunity',offset)]
  initial_equal=ar['full_native_row']['initial']==br['full_native_row']['initial']
  word_same=ar['record']['first16_native_language_lanes']==br['record']['first16_native_language_lanes']
  heard_same=a['first_two_heard_slots']==b['first_two_heard_slots']
  cooldown_same=a['prompted_cooldown_ready']==b['prompted_cooldown_ready']
  label_pair={a['label'],b['label']}=={'supported_recent','unsupported_observed_prefix'}
  diffs=diff(ar['full_host_before'],br['full_host_before'])
  pairs.append({'seed':seed,'offset':offset,'near_label':a['label'],'far_label':b['label'],'both_actual_matching_declarations':ar['record']['actual_matching_declare']and br['record']['actual_matching_declare'],'valid_unsupported_label_pair':label_pair,'same_actual_tick':a['tick']==b['tick'],'same_sampling_counter':ar['full_native_row']['sampling_counter']==br['full_native_row']['sampling_counter'],'same_first_two_heard_slots':heard_same,'near_heard':a['first_two_heard_slots'],'far_heard':b['first_two_heard_slots'],'same_native_word_lanes':word_same,'same_prompted_cooldown_ready':cooldown_same,'near_prompted_ready':a['prompted_cooldown_ready'],'far_prompted_ready':b['prompted_cooldown_ready'],'same_unprompted_ready':a['unprompted_cooldown_ready']==b['unprompted_cooldown_ready'],'same_pre_forward_neural_initial':initial_equal,'same_effective_speech_offsets':ar['full_native_row']['replay_tick']['effective_weight_offsets'][27648:28672]==br['full_native_row']['replay_tick']['effective_weight_offsets'][27648:28672],'changed_pre_state_paths_count':len(diffs),'changed_pre_state_path_samples':diffs[:40],'both_frozen_classifications_correct':label_pair and a['verdict']in['preserved_valid_report','suppressed_unsupported_report']and b['verdict']in['preserved_valid_report','suppressed_unsupported_report']})
# Independent check that source/far forks differ solely in initial food position.
setup=[]
for seed in plan['seeds']:
 a=rd(ROOT/f'start-{seed}-near_opportunity.json.zst');b=rd(ROOT/f'start-{seed}-far_opportunity.json.zst');paths=diff(a,b)
 objects=a['world']['world']['objects'];food=next(v['scene']['food']for v in native['matching_forks']if v['seed']==seed)
 food_index=next(i for i,v in enumerate(objects)if v['id']==food)
 refs=a['world']['world']['voxel_backend']['world_resource_refs'];ref_index=next(i for i,v in enumerate(refs)if v['stable_id']==food)
 allowed=[f'/world/world/objects/{food_index}/position/',f'/world/world/voxel_backend/world_resource_refs/{ref_index}/tile/']
 assert paths and all(any(p.startswith(prefix)for prefix in allowed)for p in paths),paths
 setup.append({'seed':seed,'exact_different_paths':paths,'only_food_position_and_its_derived_voxel_tile_changed':True})
summary={'labeled_valid_reports':sum(r['label']=='supported_recent'for r in records),'labeled_unsupported_reports':sum(r['label']=='unsupported_observed_prefix'for r in records),'abstentions':sum(r['verdict']=='abstain_unlabeled_actual_state'for r in records),'valid_reports_preserved_by_frozen_fit':sum(r['verdict']=='preserved_valid_report'for r in records),'unsupported_reports_suppressed_by_frozen_fit':sum(r['verdict']=='suppressed_unsupported_report'for r in records),'matched_valid_unsupported_pairs':sum(p['valid_unsupported_label_pair']for p in pairs),'matched_pairs_both_correct':sum(p['both_frozen_classifications_correct']for p in pairs),'matched_pair_word_and_prompted_cooldown_exact':sum(p['valid_unsupported_label_pair']and p['same_native_word_lanes']and p['same_prompted_cooldown_ready']for p in pairs),'actual_food_consumptions':sum(r['actual_food_consumption']for r in records),'actual_utterance_count':sum(len(r['actual_emitted'])for r in records),'all_rows_survive':all(r['safety']['alive']for r in records),'maximum_injury':max(r['safety']['injury']for r in records),'minimum_health':min(r['safety']['health']for r in records),'minimum_energy':min(r['safety']['energy']for r in records)}
protected=plan['protected_files'];bad=[p for p,h in protected.items()if sha(p)!=h];assert not bad,bad
assert all(sha(p)==h for p,h in all_input_hashes.items())
result={'kind':'frozen-v27-separator-on-sixteen-native-v28-captures','no_refitting_no_installed_weights_no_optimizer_updates':True,'frozen_fit_margin':0.1,'frozen_fit_sha256':sha(FIT),'actor_compressed_sha256':sha(ACTOR),'native_raw_logits_all_3072_f32_values_bit_exact':all(raw_exact),'summary':summary,'records':records,'within_seed_same_offset_pairs':pairs,'initial_setup_checks':setup,'native_global_actor_and_optimizer_exact':native['all_actor_weights_moments_ages_config_mask_exact'],'protected_original_files_checked':len(protected),'all_original_files_byte_identical':not bad,'analysis_input_hashes':all_input_hashes,'limitations':['Two previously assessed V26 seeds, never used to fit the V27 LP; this is not an independent broad generalization cohort.','Natural actions, physical consequences, sensory input and private history can differ after the initial food-position intervention. Matching quality is scored rather than forced.','Only actual matching GetFood Declare emissions receive valid/unsupported labels. Other states abstain.','Frozen separator scores are single-captured-state arithmetic, not new emissions or a rollout with the separator installed.','No new fit is performed. No missing later evidence is recaptured beyond the16 allowed decisions.']}
with open(OUT/'frozen-separator-results.json','x')as f:json.dump(result,f,indent=2)
print(json.dumps({'summary':summary,'labeled_rows':[r for r in records if r['label']in['supported_recent','unsupported_observed_prefix']],'matching_pairs':[p for p in pairs if p['valid_unsupported_label_pair']],'preservation':{'checked':len(protected),'unchanged':not bad}},indent=2))
