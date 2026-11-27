"""Deterministic source regression for the actual strict schema7 dispatch edge.

Checks paths and order in pinned Rust; neither a compile nor behavior.
"""
from pathlib import Path
import re,sys
D=Path(__file__).resolve().parent
A="crates/alife_game_app/src/"
G="crates/alife_gpu_backend/src/"
W=A+"gpu_live_runtime/contact211_f0_witness.rs"
S=A+"gpu_live_runtime/staged_tick.rs"
P=A+"foundation_training_cycle/saved_life_retention/contact211_parity.rs"
R=A+"foundation_training_cycle/saved_life_retention.rs"
B=G+"closed_loop_runtime.rs"
T=G+"closed_loop_runtime/tick.rs"

def body(text, header):
    m=re.search(header, text, re.MULTILINE)
    if not m: raise ValueError("missing executable edge: "+header)
    left=text.index("{", m.end())
    depth=0
    for right in range(left,len(text)):
        if text[right]=="{": depth+=1
        elif text[right]=="}": depth-=1
        if depth==0:return text[left+1:right]
    raise ValueError("unterminated body")

def require(ok,label):
    if not ok:raise ValueError(label)

def order(text,label,*tokens):
    pos=[text.find(t) for t in tokens]
    require(-1 not in pos and pos==sorted(set(pos)),label)

def check(src):
    arm=body(src[P],r"fn arm\(")
    require("request_contact211_witness_strict(" in arm and "request_contact211_witness(" not in arm,"schema7 B arms the permissive dispatch path")
    order(arm,"A and B are explicitly disjoint","if let Some(prior)","request_contact211_witness_strict(","} else {","request_contact211_stock_reference(")
    host=body(src[W],r"fn submit_contact211_signed\(")
    prep=body(src[W],r"fn prepare_contact211_signed_submission\(")
    order(prep,"invalid strict/reference must refuse before stock selection",
          "let Some((world, prepared, before)) = cut else","return if strict || reference","Err(ScaffoldContractError::InvalidDecisionEvidence)",
          "Ok(Contact211SignedSubmission::Stock)","if reference {","Contact211SignedSubmission::Reference(world)","Contact211SignedSubmission::Strict(world, gate)")
    for t in ("[prepared], [before], [config]","request.organism == world.organism","request.decision_tick == world.decision_tick","self.world.tick() == world.decision_tick",
              "request.expected_frame == world.frame_digest","request.expected_world == world.current_world_signature","world.frame_digest == prepared.frame.frame_digest()",
              "prepared.handle == before.handle","before.tick == world.decision_tick.raw()","config.demonstrator.is_none()",
              "self.world.canonical_signature_digest().ok() == Some(world.current_world_signature)"):
        require(t in prep,"missing host signed preflight: "+t)
    order(host,"host counter or stock path may precede strict disposition","self.prepare_contact211_signed_submission(prepared, before, configs)?","if !strict {",".counter = next_counter","match dispatch")
    require(host.count(".counter = next_counter;")==2 and "Contact211SignedSubmission::Stock => return self.backend.tick_memory_batch_training(batch, configs)" in host,
            "counter paths / default stock edge changed")
    host_a=body(host,r"Contact211SignedSubmission::Reference\(world\) =>")
    order(host_a,"reference A is not actual stock","tick_memory_batch_training(batch, configs)?","Contact211Observation::Reference(world)")
    host_b=body(host,r"Contact211SignedSubmission::Strict\(world, gate\) =>")
    order(host_b,"strict host counter advances before fallible backend admission","tick_memory_batch_training_contact211_witness_strict(batch, configs, gate)?",".counter = next_counter;")
    require("tick_memory_batch_training(batch" not in host_b and "tick_memory_batch_training_contact211_witness(batch" not in host_b,"strict B can fall back to stock/permissive")
    early=body(src[W],r"fn reset_contact211_before_staged_tick\(")
    order(early,"stale strict request can be cleared to stock","self.require_contact211_signed_scope()?","self.contact211_request = None")
    require(src[S].count("self.require_contact211_signed_scope()?;")==2,"missing empty-habitat or unsigned branch guard")
    order(src[S],"strict scope may allow an empty World advance","self.require_contact211_signed_scope()?;","if self.handles.is_empty()","self.world.try_advance_tick()?;")
    order(src[S],"late canonical guard runs after sleep transaction","self.require_contact211_admitted_before_dispatch(&batch)?;","self.promote_durable_completed_sleep_batch(&completed_promotions)")
    order(src[S],"signed sampler path still writes early","let mut configs","self.submit_contact211_signed(","next_counter,", '#[cfg(not(feature = "contact211-witness"))]',".counter = next_counter;")
    b=body(src[B],r"fn tick_memory_batch_training_contact211_witness_strict\(")
    order(b,"GPU strict signature checks must propagate before submission","self.require_contact211_presample(batch, sampling, &gate)?;","self.dispatch_contact211_presample(batch, sampling, gate)")
    require("tick_memory_batch_training" not in b and ".or_else" not in b,"GPU strict path delegates to permissive/stock")
    g=body(src[B],r"fn require_contact211_presample\(")
    for t in ("let [member] = batch.members.as_slice()","gate.handle != member.handle","gate.tick != member.frame.tick()",
              "gate.frame_digest != member.frame.frame_digest()","gate.world_signature_words == [0; 4]",
              "member.frame.candidates().is_empty()","member.frame.candidates().len() > 32","sampling.len() != 1","sampling[0].demonstrator.is_some()",
              "self.next_dispatch_generation != gate.next_dispatch_generation","resident.active_weight_generation","Some(gate.expected_weight_generation)",
              "return Err(ScaffoldContractError::InvalidDecisionEvidence);"):
        require(t in g,"missing GPU signed preflight: "+t)
    record=body(src[B],r"fn dispatch_contact211_presample\(")
    require(record.count("tick_inputs_with_selector_diagnostic_capture(")==1 and "None, None, Some(sampling), true)?" in record
            and "tick_memory_batch_training" not in record,"strict dispatch is not one observer-only signed submission")
    require("let capture_logits = capture_logits || sampling.is_some();" in src[T],"signed path must request selector readback")
    order(src[T],"witness readback not prepared before selecting native recorder","if capture_logits {",
          "dispatch.selector_readback =","Some(readback) if capture_contact211_presample")
    order(src[R],"retention allows a physical error or no-progress retry",
          'if calls != 0 { return Err("Contact211 refuses to retry',"arm.arm(&mut runtime, id, before_tick)?","calls += 1;",
          "if contact211.is_some() {","GpuLiveTickOutcome::NoProgress(reason)","failed without a retry",'let tick_error =')
    return "strict schema7 source dispatch contract accepted"

def sources(root):
    return {k:(root/k).read_text() for k in (W,S,P,R,B,T)}

if __name__=="__main__":
    if len(sys.argv)!=2:raise SystemExit("usage: contact211-strict-source-check.py RECONSTRUCTED_SOURCE_ROOT")
    current=sources(Path(sys.argv[1]))
    print(check(current))
    original_call="let (ticks, proof) = self.backend.tick_memory_batch_training_contact211_witness_strict(batch, configs, gate)?;"
    host_counter='self.training_sampling.as_mut().expect("training enabled").counter = next_counter;'
    mutations={
        "fc0c-equivalent permissive request":(current,P,"request_contact211_witness_strict(Contact211WitnessRequest", "request_contact211_witness(Contact211WitnessRequest"),
        "host wrong arity stock fallback":(current,W,"return if strict || reference","return if reference"),
        "host premature counter":(current,W,original_call+"\n                "+host_counter,host_counter+"\n                "+original_call),
        "host stale mode cleared":(current,W,"        self.require_contact211_signed_scope()?;\n        self.contact211_captured","        self.contact211_captured"),
        "GPU stale gate fallback":(current,B,"self.require_contact211_presample(batch, sampling, &gate)?;","if self.require_contact211_presample(batch, sampling, &gate).is_err() { return self.tick_memory_batch_training(batch, sampling).map(|ticks| (ticks, None)); }"),
        "GPU stale weight generation ignored":(current,B,".map(|resident| resident.active_weight_generation) != Some(gate.expected_weight_generation)",".map(|resident| resident.active_weight_generation).is_none()"),
        "retention no-progress retry":(current,R,"GpuLiveTickOutcome::NoProgress(reason)","GpuLiveTickOutcome::NoProgress(_reason)"),
    }
    for label,(snapshot,key,match,repl) in mutations.items():
        case=snapshot.copy()
        if key:
            require(match in case[key],"mutation source missing: "+label)
            case[key]=case[key].replace(match,repl,1)
        try:check(case)
        except ValueError as why:print(f"DENIED {label}: {why}")
        else:raise SystemExit("regression unexpectedly accepted "+label)
