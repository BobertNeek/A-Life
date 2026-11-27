#!/usr/bin/env python3
"""Static v2 schema/geometry checks plus deterministic plan export. Never invoke evaluator."""
import json, math
from pathlib import Path
from hashlib import sha256
from collections import deque
D=Path(__file__).resolve().parent
def J(n):return json.loads((D/n).read_bytes())
def H(n):return sha256((D/n).read_bytes()).hexdigest()
C=J('qualia-v2.json'); F=J('fixture-recipes.json'); W=J('woodland-height-recipes.json')
assert C['fixture_recipes_sha256']==H('fixture-recipes.json') and C['woodland_recipes_sha256']==F['woodland_file_sha256']==H('woodland-height-recipes.json')
assert len(C['cases'])==len(F['tasks'])==8 and len(C['frozen_v1']['frozen_case_ids'])==29
assert sum(C['domain_weights_pct'].values())==100 and C['domain_weights_pct']['frozen_v1']==60
assert all(x['status']=='unqualified-do-not-score' for x in F['tasks'])
assert C['status']=='draft-not-approved-not-active' and not F['runnable_in_pinned_evaluator']
# Orthogonal native grid preflight: conservative horizontal circle clearance and ordered in-footprint gates.
# Not a movement validator (swept hull, actual canonical pose/height and locomotion are authorized elsewhere).
step=.05
geometry={}
for nm,v in W['terrain'].items():
 w=v['ground']; heights=w['heights_m']; assert (w['width'],w['depth'],len(heights))==(33,33,1089)
 assert heights[16+16*33]==0
 dx=max(abs(heights[z*33+x+1]-heights[z*33+x])/w['spacing_m']for z in range(33) for x in range(32))
 dz=max(abs(heights[(z+1)*33+x]-heights[z*33+x])/w['spacing_m']for z in range(32)for x in range(33))
 assert math.hypot(dx,dz)<.45
 gates=[(round(g['forward_m']/step),g['left_open_interval_m']) for g in v['ordered_gates']]
 assert len(gates)==2 and gates[0][0]<gates[1][0]
 assert all(-1.8<=ab[0]<ab[1]<=1.8 for _,ab in gates)
 goal=tuple(v['fixed_goal_local'])
 clear=lambda i,j: all(math.hypot(i*step-c['centre_local'][0],j*step-c['centre_local'][1])>c['radius_m']+.30 for c in v['trunks'])
 todo=deque([(0,0,0)]); seen={(0,0,0):None}; end=None
 assert clear(0,0)
 while todo:
  i,j,p=todo.popleft()
  if p==2 and math.hypot(i*step-goal[0],j*step-goal[1])<.30: end=(i,j,p);break
  for r,s in ((i+1,j),(i-1,j),(i,j+1),(i,j-1)):
   if not(-19<=r<=110 and -36<=s<=36 and clear(r,s)):continue
   q=p
   if r>i and q<2 and i<gates[q][0]<=r:
    a,b=gates[q][1]
    if not a<step*j<b:continue
    q+=1
   nxt=(r,s,q)
   if nxt not in seen:seen[nxt]=(i,j,p);todo.append(nxt)
 assert end is not None,(nm,'no conservatively clear marked route')
 p=end; lines=[]
 while p is not None:lines.append(p);p=seen[p]
 geometry[nm]={
  'xy_clear_path_exists':True,'grid_step_m':step,'min_margin_to_nominal_trunk_radius_m':round(min(math.hypot(a*step-t['centre_local'][0],b*step-t['centre_local'][1])-t['radius_m']for a,b,_ in lines for t in v['trunks']),4),
  'path_step_count':len(lines)-1,'max_upper_bound_height_slope':round(math.hypot(dx,dz),6),'grid_height_span_m':round(max(heights)-min(heights),6)}
r={'schema':'qualia.v2-draft.static-check.v1','active_authority_untouched':C['frozen_v1']['sha256'],'card_sha256':H('qualia-v2.json'),
   'activities':{'new_cases':8,'v1_cases_retained':29,'new_cases_ready_for_native':0,'native_processes':0,'author_overrides':0},
   'woodland_geometry_sanity':geometry,'limitations':['Horizontal geometry check is not native swept/rotated collision admission. Gates are snapped within half 0.05m grid step only for this static check; exact route uses authored f32 gate planes.', 'Rotate local recipe in world space and sample into a world-axis TerrainData height raster; bind exact terrain digest, native origin and up axes in both arms. Reject any setup that repositions retained actor or changes acquired state.', 'No script here is an evaluator adapter or output parser; v2 remains draft and cannot be passed to the frozen case-registry CLI.']} 
with (D/'static-check.json').open('x') as fo:json.dump(r,fo,indent=2,allow_nan=False)
print(json.dumps(r,indent=2))
