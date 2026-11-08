"""Closed scalar rejection validator. No arbitrary native error text is accepted."""
import report_contract as c
STAGES=('before_construction','asset_construction','life_construction','preparation','initial_binding','pair_barrier','resource_guard','host_step','step_validation','replay','projection','row_collection','safeguards','checkpoint_roundtrip','summary')
CODES=('native_operation','io','serialization','cpu_host','invariant','asset_binding','actual_replay','founder_mutation','transfer_execution','failed_transfer','meal_receipt','meal_channel','ownership','target_binding','sequence_binding','population','clock','preservation','deadline','resource','output_bound','handshake','accounting','safety_stop')
PROGRESS=c.obj(stage=c.enum(*STAGES),preparation_requested=c.integer(0,600),preparation_returned=c.integer(0,600),host_requested=c.U,host_returned=c.U,replay_requested=c.U,replay_returned=c.U,projected_steps=c.U,collected_rows=c.U)
def validate_progress(p):
 c.validate(p,PROGRESS)
 for kind in ('preparation','host','replay'):c.need(0<=p[kind+'_requested']-p[kind+'_returned']<=1,'diagnostic_counts')
 c.need(p['replay_requested']<=p['host_returned'] and p['collected_rows']<=p['projected_steps']<=p['host_returned'],'diagnostic_counts')
 return p
def parse_rejection(raw,case_id):
 x=c.parse(raw,c.obj(case_id=c.enum(case_id),gate=c.enum('native_rejected'),code=c.enum(*CODES),progress=PROGRESS))
 validate_progress(x['progress']);return x
