"""Pure source and synthetic payload-ABI checks, no private inputs or engine execution."""
from pathlib import Path
import re,struct,sys
if len(sys.argv)!=2: raise SystemExit('source root required')
q=Path(sys.argv[1])
a=q/'crates/alife_game_app/src'; g=q/'crates/alife_gpu_backend/src'
sh=(g.parent/'shaders/contact211_rank32.wgsl').read_text()
app=(a/'gpu_live_runtime/contact211_f0_witness.rs').read_text()
scope=(a/'foundation_training_cycle/saved_life_retention/contact211_parity.rs').read_text()
pipe=(g/'closed_loop_pipeline.rs').read_text()
hand=(g/'training_rollout.rs').read_text()
tests=(g/'closed_loop_runtime/tick.rs').read_text()
def guard(check,label):
 if not check: raise SystemExit('rank32 source refusal: '+label)
def verify(shader,host):
 guard('context.heard_tokens[2..].iter().any(Option::is_some)' in host and 'language_prior_neural_lanes()' in host
       and 'utterance.tokens.len() != 2' in host and 'world.hand_actually_empty' in host,
       'actual source rails and complete world-empty/address gate')
 guard('training_candidate_allowed(header,brain,i,0u)' in shader and 'training_candidate_allowed(header,brain,i,3u)' in shader,
       'stock representative/independent Hand predicates')
 guard('if ((legal & bit)!=0u && projected)' in shader and 'if (seen!=roster || count!=2u) { return; }' in shader
       and 'if (count>=2u) { return; }' in shader and 'abs(a-b)<band' in shader,'stock finite legal paired-only and alias abstain')
 p=shader.index('store_state_f32(')
 guard(shader.count('store_state_f32(')==2 and shader.index('if (seen!=roster')<p and shader.index('if (max(a,b)<band')<p
       and shader.index('if (!finite_decode(new_a)')<p and 'training_sample(' not in shader,'only two late score writes and no direct sampling')
verify(sh,app)
for label,b,c in [
 ('fake_lexical_input',app,app.replace('language_prior_neural_lanes()','synthesized_values()')),
 ('missing_native_roster',sh,sh.replace('seen!=roster || count!=2u','count==0u')),
 ('third_alias_not_refused',sh,sh.replace('count>=2u','count>=3u')),
 ('decoded_hand_not_stock',sh,sh.replace('training_candidate_allowed(header,brain,i,3u)','true')),
 ('forced_sampling',sh,sh+'\n// training_sample( not authorized'),
]:
 try:verify(c,app)if b is sh else verify(sh,c)
 except SystemExit:print('REJECTED',label)
 else:raise SystemExit('rank32 source allowed negative '+label)
needle='Some(readback) if contact211_presample => {'
block=pipe[pipe.index(needle):pipe.index('                Some(readback) => {',pipe.index(needle))]
p=[block.index(x)for x in ('record_memory_context_only','if batch.contact211_rank32','record_contact211_rank32',
  'record_selector_logit_copy(encoder, buffers, batch, readback, true)','record_stock_sample_speech')]
guard(p==sorted(set(p)) and 'start != training + 11' in pipe and 'end > capacity' in pipe
      and tests.count('active.enable_contact211_rank32(input, capacity)')==1,'actual detached pre-sampler order/tail bounds')
guard('grant.live_selection_edit != authorized_edit' in scope and 'r.evaluator_sha256 != a.evaluator_sha256' in scope
      and 'r.stock_arm_admission_sha256 == x.admission_sha256' in scope,'separate exact edit and same-binary prior stock reference')
guard('const CONTACT211_RANK32_PAYLOAD_WORDS: usize = 252;' in hand
      and 'CONTACT211_RANK32_PAYLOAD_WORDS:u32 = 252u;' in sh
      and 'words[12..28]' in hand and 'words[28..60]' in hand and 'words[60+i*6..66+i*6]' in hand,
      'exact Rust/WGSL payload slices')
# Stand-in words are sentinel ordinals, not a gameplay cue, model packet or trace.
sentinel_words=list(range(252))
packed=struct.pack('<252I',*sentinel_words)
got=struct.unpack_from('<6I',packed,4*(60+31*6))
guard(len(packed)==1008 and got==tuple(range(246,252)),'synthetic last-row ABI size/boundary')
print('rank32_public_source_only_controls=5_PASS; synthetic_252_word_abi_only; private_input_absent')
