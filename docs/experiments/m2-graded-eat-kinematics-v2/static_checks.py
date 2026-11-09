#!/usr/bin/env python3
"""Source/protocol checks only; no subprocess, native fixture, network or filesystem write."""
import ast,hashlib,io,json,math,pathlib,re,tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def check():
    tree=ast.parse((ROOT/'run_hosted.py').read_text());ns={'json':json,'re':re,'pathlib':pathlib}
    def fail(code):raise ValueError(code)
    ns['fail']=fail
    names={'strict_number','nullable_bool','project_reason','project_meal_v2','test_report','validate_activation','validate_tree_preservation','safe_path'}
    for node in tree.body:
        if isinstance(node,ast.Assign)and len(node.targets)==1 and isinstance(node.targets[0],ast.Name):
            name=node.targets[0].id
            if name in {'PLAN','MAIN','BASE','TEST_NAME'}:ns[name]=ast.literal_eval(node.value)
    ns['BRANCH']='refs/heads/qualification/'+ns['PLAN'];ns['PACKET']='docs/experiments/'+ns['PLAN'];ns['MARKER']=ns['PACKET']+'/ACTIVATE.json'
    for node in tree.body:
        if isinstance(node,ast.FunctionDef)and node.name in names:exec(compile(ast.Module(body=[node],type_ignores=[]),'pure-protocol','exec'),ns)
    m=json.loads((ROOT/'source-manifest.json').read_text());want={r['path']:r for r in m['meal_v2_overlay_files']};found={}
    with tarfile.open(fileobj=io.BytesIO((ROOT/'meal-kinematic-source-overlay.tar.gz').read_bytes()),mode='r:gz')as t:
        for x in t:
            assert x.isfile()and ns['safe_path'](x.name)and x.name not in found
            b=t.extractfile(x).read();r=want[x.name];assert len(b)==r['bytes']and hashlib.sha256(b).hexdigest()==r['sha256'];found[x.name]=b
    assert set(found)==set(want)
    contracts=found['crates/alife_training/src/cpu/meal_cue_v2_contracts.rs'].decode();assert '#[test]'not in contracts
    host=found['crates/alife_training/src/cpu/joint_episode_host_tests.rs'].decode();active=host.split('fn meal_kinematic_registered_two_root_matched_diagnostic_v2() {',1)[1]
    assert active.count('setup(true)')==1 and active.count('grouped_policy_step_observed::<ScriptedJointMechanismPolicy>')==2
    assert active.index('assert_pure_contracts_v2();')<active.index('setup(true)')
    records=[dict(sequence=i+1,source_frame_bytes=1000+i,frame_budget_bytes=32768,witness_retained=True,stored_capture_success=True,rebuild_capture_success=True,rebuild_capture_error=None,original_self_match=True,pair_match=False,pair_rejections=['Manipulation:relative_geometry_lane:3'],signed_joint_valence=v,source_consumed=i==1)for i,v in enumerate([0.0,0.08])]
    reason=dict(query_capture_success=True,query_capture_error=None,records=records,future_grip='Unknown',future_consumption='Unknown',scope='historical-frame diagnostic, no policy dispatch')
    g=dict(reason_report=reason,graded_joint_value=None,future_consumption='Unknown')
    rows=[dict(sequence=i+1,source_consumed=i==1,signed_joint_valence=v,v1_graded_weight=None,v2_meal_weight=(0.5 if i==1 else None))for i,v in enumerate([0.0,0.08])]
    v=dict(schema='meal-kinematic-matched-readouts-v2',comparison='matched-engineering-regression-not-held-out',sources=rows,v1_graded_joint_value=None,v2_meal_joint_value=0.04,v1_v2_cross_load_rejected=True,cold_readouts_equal=True,future_grip='Unknown',future_consumption='Unknown')
    h=dict(candidate_present=True,meal_matched=False,cue_error=None,cue_sequences=None,cue_valence=None,exact_error=None,exact_sequences=None,future_consumption='Unknown')
    def raw(value):return ('\nrunning 1 test\ntest '+ns['TEST_NAME']+' ... GRADED_CUE_DIAGNOSTIC '+json.dumps(g)+'\nMEAL_CUE_V2_DIAGNOSTIC '+json.dumps(value)+'\nCUE_HISTORICAL_SECOND_TARGET '+json.dumps(h)+'\nok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n').encode()
    assert ns['test_report'](raw(v))['meal_v2']['v2_meal_joint_value']==0.04
    changes=[lambda x:x.update(schema='old'),lambda x:x.update(extra=True),lambda x:x.update(future_grip='Held'),lambda x:x.update(v2_meal_joint_value=True),lambda x:x.update(v2_meal_joint_value=float('inf')),lambda x:x['sources'][1].update(v2_meal_weight=-0.1),lambda x:x['sources'][1].update(sequence=1),lambda x:x['sources'][1].update(source_consumed=False),lambda x:x.update(v2_meal_joint_value=0.9),lambda x:x['sources'].append(x['sources'][0])]
    for change in changes:
        bad=json.loads(json.dumps(v));change(bad)
        try:ns['test_report'](raw(bad))
        except ValueError:pass
        else:raise AssertionError('invalid projection accepted')
    a,b='a'*40,'b'*40;env={'GITHUB_ACTIONS':'true','GITHUB_EVENT_NAME':'push','GITHUB_REPOSITORY':'BobertNeek/A-Life','GITHUB_REF':ns['BRANCH'],'GITHUB_RUN_ATTEMPT':'1'}
    event={'before':a,'after':b,'ref':ns['BRANCH'],'created':False,'deleted':False,'forced':False}
    marker={'schema':'m2-graded-eat-kinematics-activation-v2','plan_id':ns['PLAN'],'main':ns['MAIN'],'research_base':ns['BASE'],'prepared_head':a,'seal_sha256':'s','source_manifest_sha256':'m','attempt':1,'fixed_disposable_qualification_only':True}
    assert ns['validate_activation'](env,event,b,[b,a],'A\t'+ns['MARKER']+'\n',marker,'s','m')==a
    for field in ['created','deleted','forced']:
        bad=event|{field:True}
        try:ns['validate_activation'](env,bad,b,[b,a],'A\t'+ns['MARKER']+'\n',marker,'s','m')
        except ValueError:pass
        else:raise AssertionError('bad trigger accepted')
    return {'status':'passed','rust_compilation':False,'native_execution':False,'new_lives':0,'overlay_files':len(found),'malformed_output_rejections':len(changes),'trigger_rejections':3,'single_fixture_dose_static':True}
if __name__=='__main__':print(json.dumps(check(),sort_keys=True))
