// IN-MEMORY PHASE B PROPOSAL ONLY. No independent test or physical permit.
// Caller/frame/token/source stay authentic and borrowed. Receipt copies only.
use super::*;
use alife_core::{ActionId, Confidence, DurationTicks, Intensity, WorldEntityId};
use std::cell::RefCell;

struct Dose {
    n: usize,
    negative: u64, positive: u64, banks: u64, receipts: u64,
    channel_receipts: u64, digest_calls: u64, equality_calls: u64,
    attempted: [u8; 50], rejected: [u8; 50], unobserved: [u8; 50],
    targetless: u8, extras: u8,
    physical_attempted: u8, physical_rejected: u8, physical_unobserved: u8,
    zero_probes: u64, zero_rejected: bool,
    zero_channel: Option<usize>, zero_field: Option<usize>,
    baseline_ok: bool, order_equal: bool, atomic_ok: bool,
    failure: u8, failure_channel: Option<usize>, failure_field: Option<usize>,
}
impl Dose {
    fn new(n: usize) -> Self {
        Self { n, negative:0, positive:0, banks:0, receipts:0,
            channel_receipts:0, digest_calls:0, equality_calls:0,
            attempted:[0;50], rejected:[0;50], unobserved:[0;50],
            targetless:0, extras:0, physical_attempted:0, physical_rejected:0,
            physical_unobserved:0, zero_probes:0, zero_rejected:false,
            zero_channel:None, zero_field:None, baseline_ok:false,
            order_equal:false, atomic_ok:true, failure:0,
            failure_channel:None, failure_field:None }
    }
    fn json(&self, complete: bool) -> serde_json::Value {
        serde_json::json!({
            "complete":complete,"channels":self.n,
            "negative_capture_calls":self.negative,"positive_copied_capture_calls":self.positive,
            "bank_clones":self.banks,"whole_receipt_clones":self.receipts,
            "additional_channel_receipt_clones":self.channel_receipts,
            "returned_bundle_digest_calls":self.digest_calls,
            "full_bank_equality_calls":self.equality_calls,
            "adapted_attempted_masks":&self.attempted[..],
            "adapted_rejected_masks":&self.rejected[..],
            "adapted_unobserved_masks":&self.unobserved[..],
            "targetless_channel_mask":self.targetless,"extra_rejected_mask":self.extras,
            "physical_attempted_mask":self.physical_attempted,
            "physical_rejected_mask":self.physical_rejected,
            "physical_unobserved_mask":self.physical_unobserved,
            "returned_zero_search_probes":self.zero_probes,
            "returned_zero_rejected":self.zero_rejected,"returned_zero_unobserved":!self.zero_rejected,
            "returned_zero_channel":self.zero_channel,"returned_zero_field":self.zero_field,
            "authentic_copy_accepted":self.baseline_ok,
            "reversed_order_equal":self.order_equal,"all_rejections_atomic":self.atomic_ok,
            "failure_kind":self.failure,"failure_channel":self.failure_channel,
            "failure_field":self.failure_field,
            "added_caller_bundle_frame_token_host_world_clones":0,
            "bank_codec_calls":0,"added_source_world_biology_intervals":0,
            "body_basis_markers":"private_mint_provenance_only",
            "postbiology_value_proof":"not_claimed",
            "paired_caller_bundle_mutation":"excluded"
        })
    }
    fn stop(&mut self, kind:u8, channel:Option<usize>, field:Option<usize>) -> ! {
        self.failure=kind;self.failure_channel=channel;self.failure_field=field;
        println!("ACTUAL_CONTACT_CAPTURE_NEGATIVE_DOSE_V1 {}",self.json(false));
        panic!("actual-contact copied-receipt audit failed");
    }
    fn validate(&mut self) {
        if !(2..=6).contains(&self.n) { self.stop(7,None,None); }
        let all=(1u8<<self.n)-1;
        let mut a=0u64;
        for f in 0..50 {
            if (self.attempted[f]&self.unobserved[f])!=0
                || (self.attempted[f]|self.unobserved[f])!=all
                || self.rejected[f]!=self.attempted[f] { self.stop(7,None,Some(f)); }
            a+=u64::from(self.attempted[f].count_ones());
        }
        let q=u64::from(self.physical_attempted.count_ones());
        let z=self.zero_rejected as u64;
        if (self.physical_attempted&self.physical_unobserved)!=0
            || (self.physical_attempted|self.physical_unobserved)!=all
            || self.physical_rejected!=self.physical_attempted
            || self.extras!=255 || self.negative!=a+8+q+z || self.negative>315
            || self.positive!=2 || self.banks!=self.negative+2 || self.banks>317
            || self.receipts!=50*self.n as u64+9+q+z || self.receipts>316
            || self.channel_receipts!=1 || self.digest_calls!=z
            || self.equality_calls!=self.negative+1 || self.zero_probes>54
            || self.zero_channel.is_some()!=self.zero_rejected
            || self.zero_field.is_some()!=self.zero_rejected
            || !self.baseline_ok || !self.order_equal || !self.atomic_ok {
            self.stop(7,None,None);
        }
    }
}
thread_local! {
    static STATE: RefCell<(bool,Option<Dose>)> = const { RefCell::new((false,None)) };
}
pub(in crate::cpu::host) fn arm() {
    STATE.with(|s| {let mut s=s.borrow_mut();assert!(!s.0&&s.1.is_none());s.0=true;});
}
pub(in crate::cpu::host) fn take() -> serde_json::Value {
    let mut d=STATE.with(|s|s.borrow_mut().1.take()).unwrap();
    d.validate();d.json(true)
}
fn coord(v:&mut Vec3f,i:usize)->&mut f32 {
    match i {0=>&mut v.x,1=>&mut v.y,2=>&mut v.z,_=>unreachable!()}
}
fn zero_coord(v:Vec3f,i:usize)->f32 {
    match i {0=>v.x,1=>v.y,2=>v.z,_=>unreachable!()}
}
fn flip(v:&mut f32) {*v=f32::from_bits(v.to_bits()^0x8000_0000);}
fn changed(v:f32)->f32 {
    f32::from_bits(v.to_bits()^if v==0.0 {0x8000_0000}else{1})
}
fn unit_changed(v:f32)->f32 {
    if v==1.0 {f32::from_bits(v.to_bits()-1)}else{changed(v)}
}
// 0 channel;1 primitive;2 target option;3 entity option;4 entity value;
// 5 position option;6..8 position xyz;9..11 direction xyz;12 intensity;
// 13 duration;14 stand-off;15 payload length;16 confidence;17 coordination;
// 18..49 existing payload values. Absent subfields are unobserved.
fn mutate(c:&mut ChannelCommand,f:usize)->bool {
    match f {
        0=>c.channel=if c.channel==MotorChannel::Locomotion{MotorChannel::Manipulation}else{MotorChannel::Locomotion},
        1=>c.primitive=ActionId(if c.primitive.raw()==1{2}else{1}),
        2=>{if c.target.is_none(){return false}c.target=None;},
        3=>{let Some(t)=c.target.as_mut()else{return false};if t.entity.is_none(){return false}t.entity=None;},
        4=>{let Some(t)=c.target.as_mut()else{return false};let Some(e)=t.entity else{return false};t.entity=Some(WorldEntityId(if e.raw()==1{2}else{1}));},
        5=>{let Some(t)=c.target.as_mut()else{return false};if t.position.is_none(){return false}t.position=None;},
        6..=8=>{let Some(p)=c.target.as_mut().and_then(|t|t.position.as_mut())else{return false};flip(coord(p,f-6));},
        9..=11=>flip(coord(&mut c.direction,f-9)),
        12=>c.intensity=Intensity::new(unit_changed(c.intensity.raw())).unwrap(),
        13=>c.duration_ticks=DurationTicks(if c.duration_ticks.raw()==1{2}else{1}),
        14=>c.stand_off_distance=changed(c.stand_off_distance),
        15=>{if c.payload.values.is_empty(){c.payload.values.push(0);}else{c.payload.values.pop();}},
        16=>c.confidence=Confidence::new(unit_changed(c.confidence.raw())).unwrap(),
        17=>c.coordination_group^=1,
        18..=49=>{let Some(v)=c.payload.values.get_mut(f-18)else{return false};*v^=1;},
        _=>unreachable!(),
    }true
}
// Search only; no receipt copy or new value is supplied to obtain coverage.
fn is_zero(c:&ChannelCommand,f:usize)->bool {
    match f {
        0..=2=>zero_coord(c.direction,f)==0.0,
        3=>c.intensity.raw()==0.0,4=>c.stand_off_distance==0.0,
        5=>c.confidence.raw()==0.0,
        6..=8=>c.target.and_then(|t|t.position).is_some_and(|p|zero_coord(p,f-6)==0.0),
        _=>unreachable!(),
    }
}
fn flip_zero(c:&mut ChannelCommand,f:usize) {
    match f {
        0..=2=>flip(coord(&mut c.direction,f)),
        3=>c.intensity=Intensity::new(changed(c.intensity.raw())).unwrap(),
        4=>flip(&mut c.stand_off_distance),
        5=>c.confidence=Confidence::new(changed(c.confidence.raw())).unwrap(),
        6..=8=>flip(coord(c.target.as_mut().unwrap().position.as_mut().unwrap(),f-6)),
        _=>unreachable!(),
    }
}
#[allow(clippy::too_many_arguments)]
fn reject(base:&ContactBankV1,frame:&PerceptionFrame,bundle:&MotorCommandBundle,
    receipt:&alife_world::HeadlessMotorTransactionReceipt,motors:[Option<u16>;6],
    generation:u64,source:[u64;4],bindings:&ContactTargetBindingsV1,
    d:&mut Dose,kind:u8,index:Option<usize>,field:Option<usize>) {
    let mut trial=base.clone();d.banks+=1;d.negative+=1;
    let rejected=trial.capture_close(frame,bundle,receipt,motors,generation,source,bindings).is_err();
    d.equality_calls+=1;let unchanged=trial.bank==base.bank;d.atomic_ok&=unchanged;
    if !rejected||!unchanged {d.stop(kind,index,field);}
}
#[allow(clippy::too_many_arguments)]
pub(in crate::cpu::host) fn run_if_armed(base:&ContactBankV1,frame:&PerceptionFrame,
    bundle:&MotorCommandBundle,receipt:&alife_world::HeadlessMotorTransactionReceipt,
    motors:[Option<u16>;6],generation:u64,source:[u64;4],bindings:&ContactTargetBindingsV1) {
    let armed=STATE.with(|s|{let mut s=s.borrow_mut();let a=s.0;s.0=false;a});
    if !armed{return}
    let n=receipt.channel_receipts.len();let mut d=Dose::new(n);
    if !(2..=6).contains(&n)||bundle.channels.len()!=n||bundle.sequence_id.raw()!=1 {
        d.stop(7,None,None);
    }
    let mut baseline=base.clone();d.banks+=1;d.positive+=1;
    d.baseline_ok=baseline.capture_close(frame,bundle,receipt,motors,generation,source,bindings).is_ok();
    if !d.baseline_ok {d.stop(1,None,None);}
    let mut reversed=receipt.clone();d.receipts+=1;reversed.channel_receipts.reverse();
    let mut reordered=base.clone();d.banks+=1;d.positive+=1;
    let ok=reordered.capture_close(frame,bundle,&reversed,motors,generation,source,bindings).is_ok();
    d.equality_calls+=1;d.order_equal=reordered.bank==baseline.bank&&ok;
    if !d.order_equal {d.stop(2,None,None);}
    for i in 0..n {
        if receipt.channel_receipts[i].command.target.and_then(|t|t.entity).is_none(){d.targetless|=1<<i;}
        for f in 0..50 {
            let mut r=receipt.clone();d.receipts+=1;
            if !mutate(&mut r.channel_receipts[i].command,f){d.unobserved[f]|=1<<i;continue}
            d.attempted[f]|=1<<i;
            reject(base,frame,bundle,&r,motors,generation,source,bindings,&mut d,3,Some(i),Some(f));
            d.rejected[f]|=1<<i;
        }
    }
    let mut found=None;
    'search:for (i,c) in bundle.channels.iter().enumerate(){
        for f in 0..9 {d.zero_probes+=1;if is_zero(c,f){found=Some((i,f));break 'search;}}
    }
    if let Some((i,f))=found {
        let mut r=receipt.clone();d.receipts+=1;flip_zero(&mut r.bundle.channels[i],f);
        d.digest_calls+=1;
        if r.bundle.canonical_digest().ok()!=Some(bindings.bundle){d.stop(6,Some(i),Some(f));}
        reject(base,frame,bundle,&r,motors,generation,source,bindings,&mut d,6,Some(i),Some(f));
        d.zero_rejected=true;d.zero_channel=Some(i);d.zero_field=Some(f);
    }
    for k in 0..8 {
        let mut r=receipt.clone();d.receipts+=1;
        match k {
            0=>r.biology_before.body.energy=f32::from_bits(r.biology_before.body.energy.to_bits()^1),
            1=>r.biology_before.tick=Tick::new(r.biology_before.tick.raw().checked_add(1).unwrap()),
            2=>r.biology_after.tick=Tick::new(r.biology_after.tick.raw().checked_add(1).unwrap()),
            3=>r.outcome_tick=Tick::new(r.outcome_tick.raw().checked_add(1).unwrap()),
            4=>{r.channel_receipts[1]=r.channel_receipts[0].clone();d.channel_receipts+=1;},
            5=>{r.channel_receipts.remove(0);},
            6=>r.bundle.organism_id=OrganismId(if bundle.organism_id.raw()==1{2}else{1}),
            7=>r.bundle.sequence_id=ExperienceSequenceId(if bundle.sequence_id.raw()==1{2}else{1}),
            _=>unreachable!(),
        }
        reject(base,frame,bundle,&r,motors,generation,source,bindings,&mut d,4,None,Some(k));
        d.extras|=1<<k;
    }
    for (i,actual) in receipt.channel_receipts.iter().enumerate() {
        let slot=channel_slot(actual.command.channel).unwrap() as usize;
        let candidate=motors[slot].and_then(|j|frame.candidates().get(j as usize));
        // Same conservative admission predicates as the live adapter; no success,
        // phase/contact flag is ever changed to manufacture a test opportunity.
        let eligible=receipt.succeeded&&actual.phase==alife_world::ActionPhase::Completed
            &&matches!(actual.physical.contact,PhysicalContactKind::Touch|PhysicalContactKind::Consumed|PhysicalContactKind::Moved)
            &&candidate.is_some_and(|c|matches!(c.kind,ActionKind::Look|ActionKind::Inspect|ActionKind::Interact|ActionKind::Hold)
                &&c.action_id.raw()!=212&&c.target.entity.is_some()&&c.target.entity==actual.physical.target_entity)
            &&!actual.grip_transfers.iter().any(|t|matches!(t.kind,alife_world::GripTransferKind::Released|alife_world::GripTransferKind::Placed));
        if !eligible {d.physical_unobserved|=1<<i;continue}
        let mut r=receipt.clone();d.receipts+=1;r.channel_receipts[i].physical.target_entity=None;
        d.physical_attempted|=1<<i;
        reject(base,frame,bundle,&r,motors,generation,source,bindings,&mut d,5,Some(i),None);
        d.physical_rejected|=1<<i;
    }
    d.validate();
    STATE.with(|s|{let mut s=s.borrow_mut();assert!(s.1.is_none());s.1=Some(d);});
}
