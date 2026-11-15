//! Opt-in read-only fixture audit; no world steps, no neural counterfactual replay.
use super::*;
use std::io::{BufRead,Write};
#[derive(serde::Deserialize)]
struct Transport { slot_index:u16, transport_entity:WorldEntityId, target_position:Vec3f }
#[derive(serde::Deserialize)]
struct Row {
 cohort:String,row:usize,sequence:u64,sensory:alife_core::SensorySnapshot,body:alife_core::BodySnapshot,
 homeostasis:alife_core::HomeostaticSnapshot,slots:Vec<alife_core::GroundedObjectSlotV1>,transports:Vec<Transport>,
 contact:Option<WorldEntityId>,focal:Option<TrackedObjectId>,active:Option<TrackedObjectId>,
 retained_coverage:Option<CandidateOpportunityCoverageV1>,actual_candidates:Vec<ActionCandidate>,
}
#[test]
#[ignore = "requires explicitly frozen native-row input/output paths"]
fn retained_fixed_row_active_opportunity_audit() {
 let input=std::env::var("ALIFE2327_FIXED_INPUT").expect("frozen input path");
 let output=std::env::var("ALIFE2327_FIXED_OUTPUT").expect("new output path");
 let file=std::fs::OpenOptions::new().write(true).create_new(true).open(output).unwrap();
 let mut out=std::io::BufWriter::new(file);let mut count=0;
 for line in std::io::BufReader::new(std::fs::File::open(input).unwrap()).lines() {
  let row:Row=serde_json::from_str(&line.unwrap()).unwrap();
  let g=GroundedSensingFrame::from_parts_for_opportunity_audit(row.sensory,row.body,row.slots,row.transports.into_iter().map(|t|crate::GroundedObjectTransport{slot_index:t.slot_index,transport_entity:t.transport_entity,target_position:t.target_position}).collect()).unwrap();
  let plan=CandidateOpportunityCoveragePlan::new(row.sequence);let profile=SensorProfile::GroundedObjectSlotsV1;
  let(initial,ir)=enumerate_grounded_candidates_with_coverage(&g,profile,row.contact,None,false,plan,0).unwrap();
  let mut draft=PerceptionFrameDraft::new(g.sensory().organism_id,g.sensory().tick,profile,g.sensory().clone(),g.body(),row.homeostasis,initial,alife_core::SensorProfileProvenance::new(profile,alife_core::SensoryAbiVersion::CURRENT,g.sensory().tick).unwrap(),g.slots().to_vec()).unwrap();
  if let Some(r)=&ir {draft=draft.with_opportunity_coverage(r.clone()).unwrap();}
  let initialwork=ir.as_ref().map_or(0,|r|r.work_units());let binding=GroundedCandidateBinding::new(g.clone(),profile,row.contact,false).with_coverage(plan);
  let(base,receipt)=if binding.coverage_plan().is_some(){binding.candidates_for_focus_with_coverage(&draft,row.focal,initialwork).unwrap()}else{(binding.candidates_for_focus(&draft,row.focal).unwrap(),None)};
  assert_eq!(receipt,row.retained_coverage,"retained receipt {}:{}",row.cohort,row.row);
  assert_eq!(base.len(),row.actual_candidates.len());
  // Runtime first-focus routing may permute candidates. Verify every descriptor
  // and target exactly after aligning only candidate_index to the actual row.
  for actual in &row.actual_candidates {
   let mut matches=base.iter().filter(|c|c.action_id==actual.action_id&&c.target==actual.target&&c.observation==actual.observation);
   let mut c=*matches.next().expect("actual descriptor absent from reconstructed V1");assert!(matches.next().is_none());c.candidate_index=actual.candidate_index;
   assert_eq!(serde_json::to_vec(&c).unwrap(),serde_json::to_vec(actual).unwrap(),"candidate bytes {}:{}",row.cohort,row.row);
  }
  let(variant,vr,proof)=binding.with_active_approach(row.active).candidates_for_focus_with_active_opportunity(&draft,row.focal,initialwork).unwrap();
  if let(Some(a),Some(b))=(&receipt,&vr) {assert_eq!(a.work_units(),b.work_units());assert_eq!(a.contact_slot(),b.contact_slot());assert_eq!(a.focal_slot(),b.focal_slot());assert_eq!(a.phase(),b.phase());}
  for protected in [proof.protected_contact,proof.protected_focal,proof.protected_rotation].into_iter().flatten(){assert!(proof.final_selected_slots.contains(&protected));}
  let targetset=|cs:&[ActionCandidate]|cs.iter().filter_map(|c|c.target.entity.map(|e|e.raw())).collect::<BTreeSet<_>>();
  let old=targetset(&base);let new=targetset(&variant);
  serde_json::to_writer(&mut out,&serde_json::json!({"cohort":row.cohort,"row":row.row,"sequence":row.sequence,"actual_receipt_exact":true,"full_candidate_set_exact":true,"base_coverage":receipt,"variant_coverage":vr,"proof":proof,"old_targets":old,"new_targets":new,"added":new.difference(&old).copied().collect::<Vec<_>>(),"removed":old.difference(&new).copied().collect::<Vec<_>>(),"candidate_count":variant.len()})).unwrap();writeln!(out).unwrap();count+=1;
 }
 out.flush().unwrap();assert_eq!(count,2368);
}
