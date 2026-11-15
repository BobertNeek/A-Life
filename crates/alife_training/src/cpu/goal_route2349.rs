//! Staged second controller: known-goal routing over actually traversed directed links.
//! Not integrated into CP20-v1 ExploreBacktrack, not an actor or a backend maze map.
use alife_core::{ActionCandidate,GroundedObjectSlotV1,TrackedObjectId,Validate};
use serde::{Serialize,Deserialize};
use super::{observed_map2349::ObservedMap,route_policy2349};
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub struct GoalRouteDecision{pub controller_version:u16,pub mode:String,pub goal:Option<TrackedObjectId>,pub path:Vec<u64>,pub path_cost:Option<f32>,pub target_heading_body:Option<f32>,pub candidate_bias:Vec<f32>,pub work_units:u64,pub fallback:Option<route_policy2349::RoutePreference>}
fn dist(a:[f32;2],b:[f32;2])->f32{let d=[a[0]-b[0],a[1]-b[1]];(d[0]*d[0]+d[1]*d[1]).sqrt()}
fn angle(x:f32)->f32{(x+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI}
fn fallback(map:&ObservedMap,slots:&[GroundedObjectSlotV1],gaze:f32,candidates:&[ActionCandidate],work:u64)->Result<GoalRouteDecision,String>{let r=route_policy2349::preference(map,slots,gaze,candidates)?;Ok(GoalRouteDecision{controller_version:2,mode:"ExploreBacktrackFallback".into(),goal:map.goal(),path:vec![],path_cost:None,target_heading_body:r.target_heading_body,candidate_bias:r.candidate_bias.clone(),work_units:work+r.work.units()+r.candidate_bias.len()as u64+9,fallback:Some(r)})}
pub fn preference(map:&ObservedMap,slots:&[GroundedObjectSlotV1],gaze:f32,candidates:&[ActionCandidate])->Result<GoalRouteDecision,String>{
 let mut work=map.validate_counted()?;
 if candidates.len()>32||slots.len()>alife_core::MAX_GROUNDED_OBJECT_SLOTS||!gaze.is_finite(){return Err("invalid goal-route input".into())}
 let mut goal=None;for n in map.nodes(){work+=2;if Some(n.id)==map.goal(){work+=1;if map.frame_version2367()==3{work+=1;if !map.landmark_position_verified2367(n){continue}}goal=Some(n);break}}
 let Some(goal)=goal else{return fallback(map,slots,gaze,candidates,work)};
 let nodes=map.visited_nodes();let mut start=None;for(i,n)in nodes.iter().enumerate(){work+=2;if Some(n.id)==map.current_visit(){start=Some(i);break}}
 let Some(start)=start else{return fallback(map,slots,gaze,candidates,work)};
 let current_distance=dist(map.position(),goal.position);work+=1;if !current_distance.is_finite(){return fallback(map,slots,gaze,candidates,work)}
 let mut costs=vec![f32::INFINITY;nodes.len()];let mut previous:Vec<Option<usize>>=vec![None;nodes.len()];let mut done=vec![false;nodes.len()];work+=3*nodes.len()as u64;costs[start]=0.;
 for _ in 0..nodes.len(){
  let mut selected:Option<usize>=None;for i in 0..nodes.len(){work+=2;if done[i]||!costs[i].is_finite(){continue}if selected.map_or(true,|j|costs[i]<costs[j]||(costs[i]==costs[j]&&nodes[i].id<nodes[j].id)){selected=Some(i);work+=1}}
  let Some(u)=selected else{break};done[u]=true;work+=1;
  for edge in map.edges(){work+=3;if edge.from!=nodes[u].id||edge.successes==0{continue}
   let mut endpoint=None;for(i,n)in nodes.iter().enumerate(){work+=2;if n.id==edge.to{endpoint=Some(i);break}}
   let Some(v)=endpoint else{return Err("invalid traversed endpoint".into())};
   let length=dist(edge.displacement,[0.;2]);work+=1;if !length.is_finite()||length<=0.0{continue}
   // Nonnegative observed length + uncertainty + blocked-attempt penalty. This
   // first goal variant does NOT attribute joint ingestion harm to locomotion.
   let cost=length+(1.-edge.confidence()*nodes[v].confidence).clamp(0.,1.)+edge.blocked as f32/(edge.successes as f32+1.);let total=costs[u]+cost;work+=4;
   if !total.is_finite(){continue}
   if !done[v]&&(total<costs[v]||(total==costs[v]&&previous[v].map_or(true,|p|nodes[u].id<nodes[p].id))){costs[v]=total;previous[v]=Some(u);work+=2;}
  }
 }
 let mut target:Option<usize>=None;let mut best_distance=current_distance;
 for(i,n)in nodes.iter().enumerate(){work+=2;if i==start||!costs[i].is_finite(){continue}let d=dist(n.position,goal.position);work+=1;
  // Improvement must exceed one explicit loop-association/body-radius margin.
  if d+super::observed_map2349::LOOP_RADIUS>=current_distance{continue}
  if d<best_distance||target.is_some_and(|j|d==best_distance&&(costs[i],n.id)<(costs[j],nodes[j].id)){target=Some(i);best_distance=d;work+=2;}
 }
 let Some(target)=target else{return fallback(map,slots,gaze,candidates,work)};
 let mut path=vec![target];let mut cursor=target;
 while cursor!=start{work+=2;let Some(p)=previous[cursor]else{return Err("broken known route".into())};path.push(p);cursor=p;if path.len()>nodes.len(){return Err("route cycle".into())}}
 path.reverse();work+=path.len()as u64;let next=nodes[path[1]].position;let h=angle((next[1]-map.position()[1]).atan2(next[0]-map.position()[0])-map.yaw());work+=1;
 let mut biases=Vec::with_capacity(candidates.len());for(i,c)in candidates.iter().enumerate(){work+=2;c.validate_contract().map_err(|e|format!("{e:?}"))?;if c.candidate_index as usize!=i{return Err("unaligned goal-route candidate".into())}let b=match c.action_id.raw(){105=>4.*h.cos(),604=>4.*(h/std::f32::consts::PI),605=>-4.*(h/std::f32::consts::PI),_=>0.};biases.push(b);work+=3;}
 let path:Vec<_>=path.into_iter().map(|i|nodes[i].id).collect();work+=path.len()as u64+9;
 Ok(GoalRouteDecision{controller_version:2,mode:"GraphGoalRoute".into(),goal:Some(goal.id),path,path_cost:Some(costs[target]),target_heading_body:Some(h),candidate_bias:biases,work_units:work,fallback:None})
}
impl GoalRouteDecision {
 pub fn projection(&self)->route_policy2349::RoutePreference {
  if let Some(r)=&self.fallback{return r.clone()}
  route_policy2349::RoutePreference{version:1,reason:route_policy2349::RouteReason::KnownGoalRoute,target_heading_body:self.target_heading_body,observed_landmark:None,traversed_node:self.path.get(1).copied(),candidate_bias:self.candidate_bias.clone(),work:route_policy2349::RouteWork::default()}
 }
 pub fn validate_projection(&self,r:&route_policy2349::RoutePreference)->Result<(),String>{
  if self.controller_version!=2||self.path.len()>32||self.candidate_bias.len()>32||self.candidate_bias.is_empty()||!self.candidate_bias.iter().all(|x|x.is_finite()&&x.abs()<=4.)||self.target_heading_body.is_some_and(|h|!h.is_finite()||h.abs()>std::f32::consts::PI){return Err("invalid bounded goal route".into())}
  if let Some(id)=self.goal{id.validate().map_err(|e|format!("{e:?}"))?;}
  if let Some(f)=&self.fallback{if self.mode!="ExploreBacktrackFallback"||!self.path.is_empty()||self.path_cost.is_some()||f.target_heading_body!=self.target_heading_body||f.candidate_bias.iter().map(|x|x.to_bits()).collect::<Vec<_>>()!=self.candidate_bias.iter().map(|x|x.to_bits()).collect::<Vec<_>>(){return Err("invalid route fallback".into())}}
  else{if self.mode!="GraphGoalRoute"||self.goal.is_none()||self.path.len()<2||self.path_cost.is_none_or(|x|!x.is_finite()||x<=0.)||self.target_heading_body.is_none(){return Err("invalid known route".into())}for(i,id)in self.path.iter().enumerate(){if *id==0||self.path[..i].contains(id){return Err("invalid route path".into())}}}
  if serde_json::to_vec(&self.projection()).map_err(|e|e.to_string())?!=serde_json::to_vec(r).map_err(|e|e.to_string())?{return Err("goal route projection mismatch".into())}Ok(())
 }
}
