//! Asset-free tests against the genuine World and ordinary staged openings.
//! CPU command below is a test of World payment/transport, never a sampler claim.
use alife_core::{
    BiochemistryState, BrainCapacityClass, ChannelCommand, Confidence, CreatureGenome,
    DurationTicks, EgocentricDirection, ExperienceSequenceId, FoundationGeneticIdentity, HomeostaticSnapshot,
    Intensity, MotorChannel, MotorCommandBundle, OrganismId, PerceptionContextBlock,
    PhysicalContactKind, SensorProfile, Vec3f,
};
use alife_world::{HeadlessActionIds, HeadlessScenarioBuilder, WorldOrganismRecord};

fn meters(a: Vec3f, b: Vec3f) -> f64 {
    (f64::from(a.x)-f64::from(b.x)).hypot(f64::from(a.y)-f64::from(b.y))
        .hypot(f64::from(a.z)-f64::from(b.z))
}
fn bits(a: Vec3f) -> [u32;3] { [a.x.to_bits(), a.y.to_bits(), a.z.to_bits()] }

#[test]
fn actual_world_paid_move_rebinds_only_the_same_current_transport() {
    let owner=OrganismId(3);
    let mut world=HeadlessScenarioBuilder::new(9811)
        .agent("test-resident", owner, Vec3f::ZERO)
        .food("object-a", Vec3f::new(2.1,0.0,0.4), 0.6)
        .toy("object-b", Vec3f::new(2.0,0.0,-0.5), true).build().unwrap();
    let body=world.entity_id("test-resident").unwrap();
    let a=world.entity_id("object-a").unwrap();
    let b=world.entity_id("object-b").unwrap();
    let t0=world.tick();
    let genetics=FoundationGeneticIdentity::new(10,1,7,BrainCapacityClass::N512_ID).unwrap();
    let genome=CreatureGenome::early_mammal_founder(0xE10_32A1,genetics).unwrap();
    let ph=genome.express().unwrap();
    let chem=BiochemistryState::new(&ph,t0).unwrap();
    world.register_organism_record(WorldOrganismRecord::new(owner,body,genome,ph,chem,t0).unwrap()).unwrap();
    assert!(world.organism_registry().get(owner).unwrap().lifecycle().is_alive());
    let p0=world.entity(body).unwrap().position;
    let fixed=world.entity(a).unwrap().position;
    let at=world.entity(a).unwrap().tracking_key;
    let bt=world.entity(b).unwrap().tracking_key;
    assert_ne!(at,bt);
    let (draft, bind)=world.perception_frame_draft_with_bindings(owner,t0,
        SensorProfile::GroundedTerrainVisionV1,HomeostaticSnapshot::baseline(t0)).unwrap();
    let first=draft.finalize(PerceptionContextBlock::empty()).unwrap();
    let bind=bind.unwrap();
    let approach=*first.candidates().iter().find(|c|c.action_id==HeadlessActionIds::APPROACH
        && c.target.entity==Some(a)).expect("the actual observed prop must offer Approach");
    assert!(bind.matches_word_local_transport_v1(&first,&approach));
    let mut swapped=approach;
    swapped.target.entity=Some(b);
    assert!(!bind.matches_word_local_transport_v1(&first,&swapped));
    swapped.target.entity=Some(a);
    swapped.target.position=Some(world.entity(b).unwrap().position);
    assert!(!bind.matches_word_local_transport_v1(&first,&swapped));
    let cmd=ChannelCommand::new(MotorChannel::Locomotion, HeadlessActionIds::APPROACH,
        Some(approach.target),EgocentricDirection::new(1.0,0.0,0.0),Intensity::new(1.0).unwrap(),DurationTicks::new(1),0.0,
        Confidence::new(1.0).unwrap(),0).unwrap();
    let bundle=MotorCommandBundle::new(owner,ExperienceSequenceId::new(1).unwrap(),t0,vec![cmd]).unwrap();
    let actual=world.apply_registered_motor_bundle(&bundle,body).unwrap();
    assert!(actual.succeeded && actual.grip_transfers.is_empty());
    let [row]=actual.joint.channel_outcomes.as_slice() else { panic!("not one World Locomotion") };
    let paid=row.execution_evidence.as_ref().expect("actual World execution evidence");
    assert_eq!(row.channel,MotorChannel::Locomotion);
    assert!(paid.succeeded && paid.primitive==HeadlessActionIds::APPROACH
        && paid.command_target==Some(a) && row.physical.target_entity==Some(a)
        && row.physical.contact==PhysicalContactKind::Moved && row.physical.energy_cost.raw()>0.0);
    let p1=world.entity(body).unwrap().position;
    let d=row.physical.displacement;
    assert!(meters(p0,fixed)-meters(p1,fixed)>0.001);
    assert!(meters(Vec3f::new(p1.x-p0.x,p1.y-p0.y,p1.z-p0.z),d)<0.0001);
    assert_eq!(bits(world.entity(a).unwrap().position),bits(fixed));
    assert_eq!(world.entity(a).unwrap().tracking_key,at);
    assert!(world.entity(a).unwrap().carried_by.is_none());
    if world.tick()==t0 { world.try_advance_tick().unwrap(); }
    let t1=world.tick();
    assert_eq!(t1,actual.outcome_tick);
    assert_eq!(t1.raw(),t0.raw()+1);
    let (draft, next)=world.perception_frame_draft_with_bindings(owner,t1,
        SensorProfile::GroundedTerrainVisionV1,HomeostaticSnapshot::baseline(t1)).unwrap();
    let second=draft.finalize(PerceptionContextBlock::empty()).unwrap();
    let next=next.unwrap();
    let current=second.candidates().iter().find(|c|c.action_id==HeadlessActionIds::APPROACH
        && c.target.entity==Some(a)).expect("next real view retains same Approach target");
    assert!(next.matches_word_local_transport_v1(&second,current));
    assert!(!bind.matches_word_local_transport_v1(&second,current));
    assert!(!next.matches_word_local_transport_v1(&first,&approach));
    assert_eq!(&current.features.0[9..15],&approach.features.0[9..15]);
}
