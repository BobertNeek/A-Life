//! CP20 explicit innate experienced-graph comparator. Not acquired neural learning.
use super::*;
use alife_core::*;
use super::observed_map2349::{ObservedMap,MeasuredMotion};
use super::route_policy2349::{self,RoutePreference};
use serde::{Serialize,Deserialize};
fn err(s:impl Into<String>)->CpuHostError{CpuHostError::Preparation(s.into())}
fn digest<T:Serialize>(domain:&[u8],x:&T)->Result<[u64;4],CpuHostError>{Ok(digest_counted(domain,x)?.0)}
/// One source-payload byte unit covers its serialization and canonical-hash traversal
/// as a logical operation, not a count of machine instructions.
fn digest_counted<T:Serialize>(domain:&[u8],x:&T)->Result<([u64;4],u64),CpuHostError>{let bytes=serde_json::to_vec(x)?;let work=bytes.len()as u64+domain.len()as u64;let mut d=CanonicalDigestBuilder::new(domain);d.write_bytes(&bytes);Ok((d.finish256(),work))}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnPose2349{pub position:[f32;2],pub body_yaw:f32,pub head_yaw:f32}
fn own_pose(world:&alife_world::HeadlessWorld,id:OrganismId)->Result<(OwnPose2349,u64),CpuHostError>{
 let snapshots=world.object_snapshots();let mut work=1;let mut result=None;
 // One bounded WorldObject clone/visit plus label-byte copying. Other objects'
 // pose/kind/material are NEVER read or returned to the graph/controller.
 for object in &snapshots {work+=3+object.label.len()as u64;if object.organism_id==Some(id){if result.is_some(){return Err(err("duplicate own body"))}result=Some(OwnPose2349{position:[object.position.x,object.position.z],body_yaw:object.body_yaw,head_yaw:object.head_yaw});work+=4;}}
 let pose=result.ok_or_else(||err("missing own body"))?;if !pose.position.iter().all(|x|x.is_finite())||!pose.body_yaw.is_finite()||!pose.head_yaw.is_finite(){return Err(err("invalid own pose"))}Ok((pose,work+1))
}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationReceipt2349{pub version:u16,pub organism_id:OrganismId,pub sequence:ExperienceSequenceId,pub frame_digest:PerceptionFrameDigest,pub candidate_digest:[u64;4],pub map_before:[u64;4],pub map_observed:[u64;4],pub route:RoutePreference,#[serde(default,skip_serializing_if="Option::is_none")]pub goal_route:Option<super::goal_route2349::GoalRouteDecision>,pub base_bits:Vec<u32>,pub output_bits:Vec<u32>,pub work_units:u64,pub outcome_work_units:u64,pub checksum:[u64;4]}
impl NavigationReceipt2349{
 fn payload(&self)->Result<[u64;4],CpuHostError>{let mut r=self.clone();r.checksum=[0;4];digest(b"ALIFE-INNATE-GRAPH-RECEIPT-2349",&r)}
 pub fn validate(&self)->Result<(),CpuHostError>{self.organism_id.validate()?;self.sequence.validate()?;if self.route.version!=1||self.candidate_digest==[0;4]||self.map_before==[0;4]||self.map_observed==[0;4]||!matches!(self.version,1|2|3)||self.base_bits.len()!=self.output_bits.len()||self.base_bits.len()!=self.route.candidate_bias.len()||self.base_bits.is_empty()||self.base_bits.len()>32||self.frame_digest.0==[0;4]||self.checksum!=self.payload()?{return Err(err("invalid graph receipt binding"))}if (self.version==2)!=self.goal_route.is_some(){return Err(err("goal route receipt version"))}if let Some(g)=&self.goal_route{g.validate_projection(&self.route).map_err(err)?;}for((base,out),bias)in self.base_bits.iter().zip(&self.output_bits).zip(&self.route.candidate_bias){let b=f32::from_bits(*base);if !b.is_finite()||!bias.is_finite()||bias.abs()>4.||(*bias!=0.&&(b+*bias).to_bits()!=*out)||(*bias==0.&&*base!=*out){return Err(err("invalid graph bias bits"))}}Ok(())}
 pub(super) fn seal_outcome_work(&mut self,work:u64)->Result<(),CpuHostError>{self.outcome_work_units=work+64+12*self.output_bits.len()as u64;self.checksum=self.payload()?;self.validate()}
 pub fn validate_row(&self,frame:&PerceptionFrame,sequence:ExperienceSequenceId,context:&crate::TrainingReplayTick)->Result<(),CpuHostError>{
  self.validate()?;if self.organism_id!=frame.organism_id()||self.sequence!=sequence||self.frame_digest!=frame.frame_digest()||self.candidate_digest!=digest(b"ALIFE-GRAPH-CANDIDATES-2349",&frame.candidates())?||context.candidates.len()!=self.output_bits.len(){return Err(err("graph row frame binding"))}
  if let Some(id)=self.route.observed_landmark{if !frame.grounded_object_slots().iter().any(|s|s.tracked_object_id==id){return Err(err("graph route unseen landmark"))}}
  for (index,((c,r),bits)) in frame.candidates().iter().zip(&context.candidates).zip(&self.output_bits).enumerate(){let bias=self.route.candidate_bias[index];let expected=if let Some(h)=self.route.target_heading_body{if !h.is_finite()||h.abs()>std::f32::consts::PI{return Err(err("graph invalid heading"))}match c.action_id.raw(){105=>4.0*h.cos(),604=>4.0*(h/std::f32::consts::PI),605=>-4.0*(h/std::f32::consts::PI),_=>0.0}}else{0.0};let legal=matches!((c.action_id.raw(),c.kind),(105,ActionKind::Move)|(604|605,ActionKind::Look))&&c.observation==CandidateObservationRef::None&&c.target==ActionTarget::NONE;if bias.to_bits()!=expected.to_bits()||(!legal&&bias!=0.)||c.family!=r.family||r.innate_bias.to_bits()!=*bits{return Err(err("graph row replay binding"))}}
  Ok(())
 }

}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationMotion2349 {pub source:MazeBlockedHistory2346,pub displacement_world:[f32;2]}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationGoal2349{pub sequence:ExperienceSequenceId,pub frame:PerceptionFrameDigest,pub tracked_id:TrackedObjectId,pub entity:WorldEntityId}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationState2349{pub version:u16,pub organism_id:OrganismId,pub map:ObservedMap,pub previous_pose:Option<OwnPose2349>,pub pending:Option<NavigationMotion2349>,pub last_receipt:Option<NavigationReceipt2349>,pub goal_source:Option<NavigationGoal2349>}
impl NavigationState2349{
 pub fn empty(id:OrganismId)->Result<Self,CpuHostError>{id.validate()?;Ok(Self{version:1,organism_id:id,map:ObservedMap::default(),previous_pose:None,pending:None,last_receipt:None,goal_source:None})}
 pub fn validate(&self,id:OrganismId,next_sequence:u64)->Result<(),CpuHostError>{if !matches!(self.version,1|2|3)||self.organism_id!=id{return Err(err("graph resident identity"))}self.map.validate().map_err(err)?;if self.goal_source.as_ref().map(|g|g.tracked_id)!=self.map.goal(){return Err(err("graph goal source mismatch"))}if let Some(g)=&self.goal_source{g.tracked_id.validate()?;g.entity.validate()?;g.sequence.validate()?;if g.sequence.raw()>self.map.sequence()||g.frame.0==[0;4]{return Err(err("graph goal provenance"))}}if self.map.sequence()>=next_sequence||(self.map.sequence()==0)!=(self.previous_pose.is_none())||(self.map.sequence()==0)!=(self.last_receipt.is_none()){return Err(err("graph map chronology"))}if let Some(p)=&self.previous_pose{if !p.position.iter().all(|x|x.is_finite())||!p.body_yaw.is_finite()||!p.head_yaw.is_finite(){return Err(err("graph previous pose"))}}if let Some(pending)=&self.pending{let p=&pending.source;p.validate()?;if !pending.displacement_world.iter().all(|x|x.is_finite())||p.organism_id!=id||p.source_sequence.raw()>=next_sequence||self.last_receipt.as_ref().is_none_or(|r|r.sequence!=p.source_sequence||r.frame_digest!=p.source_frame){return Err(err("graph pending provenance"))}}if let Some(r)=&self.last_receipt{r.validate()?;if (self.version==3&&r.version==2)||r.version>self.version||r.organism_id!=id||r.sequence.raw()>=next_sequence{return Err(err("graph receipt chronology"))}}Ok(())}
 pub(super) fn prepare(&mut self,world:&alife_world::HeadlessWorld,frame:&PerceptionFrame,sequence:ExperienceSequenceId,p:&BrainPhenotype,context:&mut crate::TrainingReplayTick)->Result<NavigationReceipt2349,CpuHostError>{
  if self.organism_id!=frame.organism_id(){return Err(err("graph frame owner"))}
  let (pose,mut work)=own_pose(world,self.organism_id)?;let(before,before_work)=digest_counted(b"ALIFE-OBSERVED-GRAPH-2349",&self.map)?;work+=before_work;
  let mut motion=MeasuredMotion{displacement:[0.;2],yaw_delta:0.,blocked:false,zero_step_blocked:false,executed_traversal:false};
  if let Some(old)=&self.previous_pose{let dx=pose.position[0]-old.position[0];let dz=pose.position[1]-old.position[1];let(s,c)=old.body_yaw.sin_cos();motion.displacement=[dx*c+dz*s,dz*c-dx*s];motion.yaw_delta=pose.body_yaw-old.body_yaw;work+=7;}
  if let Some(pending)=self.pending.take(){let prior=pending.source;prior.validate()?;if prior.organism_id!=frame.organism_id()||prior.source_sequence.raw()>=sequence.raw()||prior.source_tick>=frame.tick(){return Err(err("future graph motion"))}motion.blocked=prior.blocked;motion.zero_step_blocked=prior.blocked&&prior.primitive.raw()==105;
   // Only an authenticated actual Locomotion displacement may create a traversed
   // edge. Own-pose differences from passive movement/transport remain odometry.
   if let Some(old)=&self.previous_pose{let net=[pose.position[0]-old.position[0],pose.position[1]-old.position[1]];motion.executed_traversal=net.map(f32::to_bits)==pending.displacement_world.map(f32::to_bits);}
   work+=14;}
  let mut optimized_route=None;
  let(map,map_work)=if self.version==3 {
   let(validated,map_work)=self.map.observe_validated(sequence,frame.grounded_object_slots(),alife_world::GROUNDED_VISION_RADIUS,pose.head_yaw,motion,None).map_err(err)?;
   optimized_route=Some(route_policy2349::preference_validated(&validated,frame.grounded_object_slots(),pose.head_yaw,frame.candidates()).map_err(err)?);
   (validated.into_inner(),map_work)
  }else{self.map.observe(sequence,frame.grounded_object_slots(),alife_world::GROUNDED_VISION_RADIUS,pose.head_yaw,motion,None).map_err(err)?};self.map=map;if self.map.goal().is_none()&&self.goal_source.take().is_some(){work+=1;}self.previous_pose=Some(pose.clone());work+=map_work.units()+2;
  let goal_route=if self.version==2{Some(super::goal_route2349::preference(&self.map,frame.grounded_object_slots(),pose.head_yaw,frame.candidates()).map_err(err)?)}else{None};
  let route=if let Some(g)=&goal_route{work+=g.work_units+g.candidate_bias.len()as u64+12;g.projection()}else{let r=if let Some(r)=optimized_route{r}else{route_policy2349::preference(&self.map,frame.grounded_object_slots(),pose.head_yaw,frame.candidates()).map_err(err)?};work+=r.work.units();r};
  if context.candidates.len()!=frame.candidates().len(){return Err(err("graph candidate alignment"))}
  let(mut bases,mut outputs)=(Vec::new(),Vec::new());
  for((dst,c),bias)in context.candidates.iter_mut().zip(frame.candidates()).zip(&route.candidate_bias){
   let family=p.candidate_decoder().families().iter().find(|f|{work+=1;f.family()==c.family}).ok_or_else(||err("graph family"))?;
   let base=family.innate_contribution(frame.homeostasis().drives,c.features);work+=5;
   if dst.family!=c.family||dst.innate_bias.to_bits()!=base.to_bits()||(!matches!(c.action_id.raw(),105|604|605)&&*bias!=0.){return Err(err("graph original innate source mismatch"))}
   bases.push(base.to_bits());if *bias!=0.{dst.innate_bias+=*bias;}outputs.push(dst.innate_bias.to_bits());work+=4;
  }
  // Hashing tariff counts serialized source bytes, plus bounded receipt field writes.
  let(candidate_digest,candidate_hash_work)=digest_counted(b"ALIFE-GRAPH-CANDIDATES-2349",&frame.candidates())?;let(map_observed,map_hash_work)=digest_counted(b"ALIFE-OBSERVED-GRAPH-2349",&self.map)?;work+=candidate_hash_work+map_hash_work+14;
  // Receipt checksum/validation/cloning: bounded field and candidate entries,
  // separate from variable-size source map/candidate byte hashing above.
  work+=64+12*context.candidates.len()as u64;
  if let Some(g)=&goal_route{work+=48+8*g.path.len()as u64+12*g.candidate_bias.len()as u64;}
  let mut receipt=NavigationReceipt2349{version:self.version,organism_id:frame.organism_id(),sequence,frame_digest:frame.frame_digest(),candidate_digest,map_before:before,map_observed,route,goal_route,base_bits:bases,output_bits:outputs,work_units:work,outcome_work_units:0,checksum:[0;4]};
  receipt.checksum=receipt.payload()?;receipt.validate()?;self.last_receipt=Some(receipt.clone());Ok(receipt)
 }
 pub(super) fn after_actual(&mut self,frame:&PerceptionFrame,sequence:ExperienceSequenceId,action:&crate::PpoJointAction,joint:&JointPhysicalOutcome)->Result<u64,CpuHostError>{
  let mut work=0;let (source,w)=MazeBlockedHistory2346::from_outcome(frame,sequence,joint)?;work+=w;
  self.pending=if let Some(source)=source{let mut displacement=None;for ch in &joint.channel_outcomes{work+=2;if ch.channel==MotorChannel::Locomotion{if displacement.is_some(){return Err(err("duplicate graph motion"))}displacement=Some([ch.physical.displacement.x,ch.physical.displacement.z]);work+=2;}}Some(NavigationMotion2349{source,displacement_world:displacement.ok_or_else(||err("missing graph displacement"))?})}else{None};work+=3;
  // Nomination follows actual selected locomotion; no future action enters prepare.
  if let Some(index)=action.motor_candidates[0]{let c=frame.candidates().get(index as usize).ok_or_else(||err("graph selected index"))?;work+=2;if c.action_id.raw()==101{if let CandidateObservationRef::ObjectSlot(slot)=c.observation{let id=frame.grounded_object_slots().get(slot as usize).ok_or_else(||err("graph selected slot"))?.tracked_object_id;let(map,w)=self.map.record_actual_approach(sequence,frame.grounded_object_slots(),id).map_err(err)?;self.map=map;self.goal_source=Some(NavigationGoal2349{sequence,frame:frame.frame_digest(),tracked_id:id,entity:c.target.entity.ok_or_else(||err("goal missing entity"))?});work+=w.units()+5;}}}
  for ch in &joint.channel_outcomes{work+=2;if ch.physical.contact!=PhysicalContactKind::Consumed{continue}
   if let Some(goal)=self.goal_source.clone(){work+=5;if ch.physical.target_entity==Some(goal.entity){let(map,w)=self.map.release_consumed_goal(sequence,goal.tracked_id).map_err(err)?;self.map=map;self.goal_source=None;work+=w.units()+2;}}
  }
  Ok(work)
 }
}
