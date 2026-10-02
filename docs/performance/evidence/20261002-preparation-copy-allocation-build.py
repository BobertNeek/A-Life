"""One-off CPU allocation comparison using emitted release artifacts."""
import hashlib,json,pathlib,subprocess,sys
root=pathlib.Path.cwd()
artifacts,exe=map(pathlib.Path,sys.argv[1:])
fixture=root/'crates/alife_core/tests/candidate_memory_retrieval.rs'
app=(root/'crates/alife_game_app/src/gpu_live_runtime.rs').read_text()
legacy=(root/'crates/alife_game_app/src/gpu_live_runtime/copy_contract_tests.rs').read_text()
helpers=app[app.index('fn route_focal_candidates('):app.index('fn cognitive_context_with_projection(')]
frozen=legacy[legacy.index('pub(super) fn legacy_route_focal_candidates('):legacy.index('#[test]')].replace('pub(super) fn','fn')
probe=r'''use alife_core::*;
use alife_core::cognitive_context::CognitiveContextFrame;
use alife_world::grounded_peripheral_summaries;
use std::alloc::{GlobalAlloc,Layout,System};
use std::sync::atomic::{AtomicBool,AtomicU64,Ordering::Relaxed};
struct Counter;
static COUNT:AtomicBool=AtomicBool::new(false);
static ALLOCS:AtomicU64=AtomicU64::new(0);
static BYTES:AtomicU64=AtomicU64::new(0);
unsafe impl GlobalAlloc for Counter {
 unsafe fn alloc(&self,l:Layout)->*mut u8 {if COUNT.load(Relaxed){ALLOCS.fetch_add(1,Relaxed);BYTES.fetch_add(l.size() as u64,Relaxed);}unsafe{System.alloc(l)}}
 unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}
 unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8 {if COUNT.load(Relaxed){ALLOCS.fetch_add(1,Relaxed);BYTES.fetch_add(n as u64,Relaxed);}unsafe{System.realloc(p,l,n)}}
}
#[global_allocator] static ALLOCATOR:Counter=Counter;
#[allow(dead_code,unused_imports)] mod fixture {
 include!(FIXTURE);
 pub fn draft()->PerceptionFrameDraft {
  let base=grounded_draft(0.4);
  let candidates=(0..26).map(|i|{let mut c=base.candidates()[i%base.candidates().len()];c.candidate_index=i as u16;c}).collect();
  PerceptionFrameDraft::new(base.organism_id(),base.tick(),base.sensor_profile(),base.sensory().clone(),base.body(),*base.homeostasis(),candidates,base.profile_provenance(),base.grounded_object_slots().to_vec()).unwrap()
 }
}
HELPERS
FROZEN
fn main(){
 let draft=fixture::draft();let sequence=ExperienceSequenceId(1);
 let context=CognitiveContextFrame::empty(draft.organism_id(),sequence,draft.tick()).unwrap();
 let summaries=grounded_peripheral_summaries(draft.grounded_object_slots()).unwrap();
 let attention=select_focal_targets(draft.organism_id(),sequence,draft.tick(),&summaries,HysteresisState::default(),AttentionSelectionPolicy::default()).unwrap();
 for rows in [8,16,32,50] {
  let mut reference=None;
  for implementation in 0..2 {
   let owned:Vec<_>=(0..rows).map(|_|(context.clone(),attention.clone())).collect();
   ALLOCS.store(0,Relaxed);BYTES.store(0,Relaxed);COUNT.store(true,Relaxed);
   let actual:Vec<_>=owned.into_iter().map(|(context,attention)|if implementation==0{
    (legacy_route_focal_candidates(draft.clone(),&attention).unwrap(),legacy_cognitive_context_with_attention(context,attention).unwrap())
   }else{(route_focal_candidates(&draft,&attention).unwrap(),cognitive_context_with_attention(context,attention).unwrap())}).collect();
   COUNT.store(false,Relaxed);
   println!("implementation={} rows={} allocations={} requested_bytes={}",implementation,rows,ALLOCS.load(Relaxed),BYTES.load(Relaxed));
   if implementation==0{reference=Some(actual);}else{assert_eq!(reference.as_ref().unwrap(),&actual);}
  }
 }
}
'''.replace('FIXTURE',json.dumps(str(fixture))).replace('HELPERS',helpers).replace('FROZEN',frozen)
out=exe.with_suffix('.rs');out.write_text(probe)
items=[json.loads(x) for x in artifacts.read_text().splitlines() if x.startswith('{')]
libs={}
for item in items:
 if item.get('reason')=='compiler-artifact' and 'lib' in item['target']['kind']:
  for f in item['filenames']:
   if f.endswith('.rlib'):libs[item['target']['name']]=f
cmd=['rustc','--edition=2021','-O','-C','debug-assertions=no',str(out),'-L','dependency='+str(pathlib.Path(libs['alife_core']).parent),'-o',str(exe)]
for name in ['alife_core','alife_world','serde_json']:cmd+=['--extern',name+'='+libs[name]]
subprocess.run(cmd,check=True)
print(json.dumps({'source_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'libraries':{n:{'path':libs[n],'sha256':hashlib.sha256(pathlib.Path(libs[n]).read_bytes()).hexdigest()} for n in ['alife_core','alife_world','serde_json']},'app_sha256':hashlib.sha256((root/'crates/alife_game_app/src/gpu_live_runtime.rs').read_bytes()).hexdigest(),'frozen_helpers_sha256':hashlib.sha256(frozen.encode()).hexdigest(),'fixture_sha256':hashlib.sha256(fixture.read_bytes()).hexdigest()}))
