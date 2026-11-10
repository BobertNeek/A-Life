import json,sys,math
p,seed,safe,side,order,palette,gain,mode=sys.argv[1:]
with open(p) as f:d=json.load(f)
assert d['schema']=='cue-study-pilot-v1' and d['seed']==int(seed)
assert d['safe_red']==(safe=='red') and d['red_left']==(side=='left') and d['safe_first']==(order=='safe-first')
assert d['matched_palette']==(palette=='matched') and d['sham']==(mode=='sham') and not d['decoder_trace_reset']
assert abs(d['approach_scale']-.7)<1e-5 and len(d['approach_edits'])==2
assert abs(d['memory_pain_gain']-(4 if gain=='pain4' else 0))<1e-5
assert len(d['memory_pain_edits'])==(48 if gain=='pain4' else 0)
assert isinstance(d['exposures'],list) and 0 < len(d['exposures']) <= 4
assert d.get('censored') or ('choice' in d and 'cold_next_exact' in d and 'founder_optimizer_unchanged' in d)
