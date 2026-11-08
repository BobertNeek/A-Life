"""Static orchestration tests. No Cargo, native life, network or private snapshots."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import hosted_transfer as h

class HostedSourceTests(unittest.TestCase):
    def test_frozen_source_and_episode_patch_bindings(self):
        m=h.read(h.HOSTED/'source-manifest.json')
        self.assertEqual(m['base_commit'],'f4cdbeed5147de0b52f95d1e4b484d7e5f03e0dc')
        self.assertEqual(m['reconstruction_sha256'],'e100b5f6febee8d17e8177e39744167391aca24c4cfbf34b3995b4a8b2514e62')
        self.assertEqual(h.file_sha(h.HOSTED/'frozen-study.patch'),m['reconstruction_sha256'])
        self.assertEqual(set(h.patch_paths((h.HOSTED/'frozen-study.patch').read_bytes())),set(m['reconstruction_paths']))
        self.assertEqual(len(m['reconstruction_paths']),30)
        self.assertEqual(h.file_sha(h.HOSTED/'episode-only.patch'),m['episode_patch_sha256'])
        self.assertEqual(h.patch_paths((h.HOSTED/'episode-only.patch').read_bytes()),m['episode_paths'])
        repo=Path(__file__).resolve().parents[3]
        for p,digest in m['episode_inputs'].items(): self.assertEqual(h.file_sha(repo/p),digest)
        self.assertEqual(len(m['frozen_inputs']),476)
        self.assertIn('crates/alife_world/src/headless.rs',m['frozen_inputs'])
        self.assertIn('crates/alife_core/src/memory/candidate_recall.rs',m['frozen_inputs'])
        self.assertIn('Cargo.lock',m['frozen_inputs'])
        self.assertEqual(len(m['builtin_asset_inputs']),4)
        self.assertEqual(len(m['workspace_manifest_inputs']),12)
    def test_closed_build_inputs_reject_mutation_or_extra_file(self):
        root=Path(tempfile.mkdtemp(prefix='synthetic-transfer-source-'))
        for c in ('alife_storage','alife_core','alife_world','alife_gpu_backend','alife_runtime','alife_training'):
            (root/'crates'/c).mkdir(parents=True)
        p='crates/alife_core/input.rs';(root/p).write_text('source')
        manifest={'frozen_inputs':{p:h.sha(b'source')},'episode_inputs':{p:h.sha(b'episode')}}
        h.verify_source(root,manifest)
        (root/p).write_text('episode');h.verify_source(root,manifest,True)
        with self.assertRaises(RuntimeError):h.verify_source(root,manifest)
        (root/'crates/alife_core/extra.rs').write_text('extra')
        with self.assertRaises(RuntimeError):h.verify_source(root,manifest,True)
    def test_exact_eight_assignment_and_hard_limits(self):
        p=h.read(h.HOSTED/'transfer-plan.json')
        self.assertEqual(p['max_lives'],8);self.assertEqual(p['max_host_ticks_per_life'],256)
        self.assertEqual(p['max_a_consumptions'],3)
        self.assertEqual(p['episodes'],[{'seed':2026111201+i,'mirror_z':bool(i%2)} for i in range(4)])
        self.assertEqual(p['construction_seed'],2026110901)
    def test_workflow_cannot_dispatch_push_cache_matrix_or_upload_private(self):
        repo=Path(__file__).resolve().parents[3]
        text=(repo/'.github/workflows/episodic-reader-transfer.yml').read_text()
        self.assertNotIn('workflow_dispatch:',text);self.assertNotIn('\n  push:',text)
        self.assertNotIn('matrix:',text);self.assertNotIn('actions/cache',text)
        self.assertIn("if: ${{ false && always()",text)
        self.assertIn("github.event.action == 'opened' && github.run_attempt == 1",text)
        self.assertIn("github.event.pull_request.number == 300",text)
        self.assertIn("github.repository == 'BobertNeek/A-Life'",text)
        self.assertIn("github.event.pull_request.head.repo.full_name == 'BobertNeek/A-Life'",text)
        self.assertIn("github.event.pull_request.head.ref == 'experiment/episodic-reader-transfer-20261107'",text)
        self.assertIn('TRANSFER_STUDY_ID: '+h.STUDY_ID,text)
        self.assertEqual(text.count('uses: actions/upload-artifact@'),1)
        uploads=text.split('name: Upload only the seven')[1].split('path: |')[1].split('if-no-files-found:')[0]
        self.assertNotIn('*',uploads)
        self.assertEqual({Path(line.strip()).name for line in uploads.splitlines() if line.strip()},set(h.PUBLIC_FILES))


class HostedPublicationTests(unittest.TestCase):
    def setUp(self):
        from unittest.mock import patch
        import os
        self.environment=patch.dict(os.environ,{'EXPECTED_SOURCE_REVISION':''})
        self.environment.start();self.addCleanup(self.environment.stop)

    def root(self):return Path(tempfile.mkdtemp(prefix='synthetic-hosted-metadata-'))
    def test_not_started_qualification_failure_emits_only_allowlisted_files(self):
        import contextlib,io
        root=self.root();self.assertFalse(h.execute(root,Path.cwd(),'a'*40,'failure'))
        log=io.StringIO()
        with contextlib.redirect_stdout(log):h.finalize(root,'a'*40,'failure')
        h.validate_public(root/'public')
        checks=h.read(root/'public/checks.json')
        self.assertFalse(checks['exposure_started']);self.assertEqual(checks['failure_stage'],'qualification')
        self.assertEqual(h.read(root/'public/summary.json')['status'],'not_started')
        self.assertIn('BEGIN_SANITIZED_TRANSFER_summary.json',log.getvalue())
    def test_unknown_metadata_or_file_cannot_upload(self):
        import contextlib,io
        root=self.root();h.execute(root,Path.cwd(),'a'*40,'failure')
        with contextlib.redirect_stdout(io.StringIO()):h.finalize(root,'a'*40,'failure')
        checks=h.read(root/'public/checks.json');checks['checkpoint']={'secret':'forbidden'};h.write(root/'public/checks.json',checks)
        with self.assertRaises(RuntimeError):h.validate_public(root/'public')
        (root/'public/raw.jsonl').write_text('not permitted')
        with self.assertRaises(RuntimeError):h.validate_public(root/'public')
    def test_prepare_failure_keeps_last_valid_not_started_report(self):
        import contextlib,io
        root=self.root();h.execute(root,Path.cwd(),'a'*40,'failure')
        checks=h.read(root/'public/checks.json');checks['failure_stage']='prepare';h.write(root/'public/checks.json',checks)
        (root/'private').mkdir();(root/'private/partial-asset').write_text('synthetic input')
        with contextlib.redirect_stdout(io.StringIO()):h.finalize(root,'a'*40,'failure')
        self.assertEqual(h.read(root/'public/checks.json')['failure_stage'],'prepare')
        h.validate_public(root/'public')

class HostedPartialRecoveryTests(unittest.TestCase):
    def setUp(self):
        from unittest.mock import patch
        import os
        self.environment=patch.dict(os.environ,{'EXPECTED_SOURCE_REVISION':''})
        self.environment.start();self.addCleanup(self.environment.stop)

    def test_completed_native_case_after_reporting_failure_is_independently_reconciled(self):
        import contextlib,io
        import test_transfer_reporting as fixture
        root=Path(tempfile.mkdtemp(prefix='synthetic-report-reconcile-'))
        private=root/'private';private.mkdir();public=root/'public'
        checks,_=h.initial_metadata(public,'a'*40,'success')
        fixture.write_json(private/'manifest.json',fixture.manifest())
        assignments=fixture.reporting.ASSIGNMENTS[:2]
        for index,(seed,arm,mirror) in enumerate(assignments):
            events,summary=fixture.synthetic_life(seed,arm,mirror)
            folder=private/f'episode-{seed}';folder.mkdir(exist_ok=True)
            (folder/f'{arm}.learning.jsonl').write_text('\n'.join(json.dumps(r) for r in events)+'\n')
            fixture.write_json(folder/f'{arm}.learning.summary.json',summary)
            if index==0:h.report(private,public)
        checks.update(status='infrastructure_failure',exposure_started=True,failure_stage='report',failure_code='sanitizer_schema',
                      attempted_cases=[{'seed':s,'arm':a} for s,a,_ in assignments],completed_cases=[{'seed':assignments[0][0],'arm':assignments[0][1]}])
        h.write(public/'checks.json',checks)
        bind_synthetic_ready(root)
        with contextlib.redirect_stdout(io.StringIO()):h.finalize(root,'a'*40,'success')
        result=h.read(public/'checks.json')
        self.assertEqual(result['status'],'infrastructure_failure');self.assertEqual(result['failure_code'],'sanitizer_schema')
        self.assertEqual(len(result['completed_cases']),2)
        self.assertEqual(h.read(public/'summary.json')['completed_lives'],2)
        h.validate_public(public)

    def test_host_error_summary_is_observed_failed_not_successful_or_lost(self):
        import contextlib,io
        import test_transfer_reporting as fixture
        root=Path(tempfile.mkdtemp(prefix='synthetic-host-error-summary-'))
        private=root/'private';private.mkdir();public=root/'public'
        checks,_=h.initial_metadata(public,'a'*40,'success')
        fixture.write_json(private/'manifest.json',fixture.manifest())
        seed,arm,mirror=fixture.reporting.ASSIGNMENTS[0]
        events,summary=fixture.synthetic_life(seed,arm,mirror)
        summary['stop_reason']='host_error: SYNTHETIC_PRIVATE_ERROR_DO_NOT_PUBLISH'
        folder=private/f'episode-{seed}';folder.mkdir()
        (folder/f'{arm}.learning.jsonl').write_text('\n'.join(json.dumps(r) for r in events)+'\n')
        fixture.write_json(folder/f'{arm}.learning.summary.json',summary)
        checks.update(status='infrastructure_failure',exposure_started=True,failure_stage='life',failure_code='native_case',
                      attempted_cases=[{'seed':seed,'arm':arm}])
        h.write(public/'checks.json',checks)
        bind_synthetic_ready(root)
        log=io.StringIO()
        with contextlib.redirect_stdout(log):h.finalize(root,'a'*40,'success')
        c=h.read(public/'checks.json')
        self.assertEqual(len(c['observed_cases']),1);self.assertEqual(c['completed_cases'],[])
        self.assertEqual(c['failed_cases'],[{'seed':seed,'arm':arm}])
        self.assertNotIn('SYNTHETIC_PRIVATE_ERROR',log.getvalue())
        h.validate_public(public)


def bind_synthetic_ready(root):
    """Synthetic metadata binding only; never claims a real build or life."""
    m=h.read(h.HOSTED/'source-manifest.json');public=root/'public';private=root/'private'
    checks=h.read(public/'checks.json');checks['source_bound']=True;checks['native_build_completed']=True;h.write(public/'checks.json',checks)
    receipt=h.read(public/'build-receipt.json');inputs=dict(m['frozen_inputs']);inputs.update(m['episode_inputs'])
    receipt.update(bound_input_count=len(inputs),effective_inputs_sha256=h.sha(json.dumps(inputs,sort_keys=True,separators=(',', ':')).encode()),rustc='rustc 1.99.1 synthetic fixture',executable_sha256='b'*64)
    h.write(public/'build-receipt.json',receipt)
    manifest=h.read(private/'manifest.json');manifest['retrieval_source_sha256']=m['retrieval_source_sha256'];h.write(private/'manifest.json',manifest)
    for p in private.rglob('*.summary.json'):
        summary=h.read(p);summary['retrieval_source_sha256']=m['retrieval_source_sha256'];h.write(p,summary)
    for p in private.rglob('*.jsonl'):
        rows=[json.loads(line) for line in p.read_text().splitlines()];rows[0]['retrieval_source_sha256']=m['retrieval_source_sha256']
        p.write_text('\n'.join(json.dumps(row) for row in rows)+'\n')

class HostedCrossBindingTests(unittest.TestCase):
    def setUp(self):
        from unittest.mock import patch
        import os
        self.environment=patch.dict(os.environ,{'EXPECTED_SOURCE_REVISION':''})
        self.environment.start();self.addCleanup(self.environment.stop)
    def ready(self):
        import contextlib,io
        import test_transfer_reporting as fixture
        root=Path(tempfile.mkdtemp(prefix='synthetic-cross-binding-'));private=root/'private';private.mkdir();public=root/'public'
        checks,_=h.initial_metadata(public,'a'*40,'success');fixture.write_json(private/'manifest.json',fixture.manifest())
        seed,arm,mirror=fixture.reporting.ASSIGNMENTS[0];events,summary=fixture.synthetic_life(seed,arm,mirror)
        folder=private/f'episode-{seed}';folder.mkdir();(folder/f'{arm}.learning.jsonl').write_text('\n'.join(json.dumps(r) for r in events)+'\n');fixture.write_json(folder/f'{arm}.learning.summary.json',summary)
        checks.update(status='infrastructure_failure',failure_stage='report',failure_code='sanitizer_schema',exposure_started=True,attempted_cases=[{'seed':seed,'arm':arm}]);h.write(public/'checks.json',checks)
        bind_synthetic_ready(root)
        with contextlib.redirect_stdout(io.StringIO()):h.finalize(root,'a'*40,'success')
        return root
    def rehash(self,public):
        (public/'SHA256SUMS').write_text(''.join(h.file_sha(public/name)+'  '+name+'\n' for name in h.PUBLIC_FILES if name!='SHA256SUMS'))
    def test_stale_receipt_is_rejected_even_with_fresh_checksum_file(self):
        root=self.ready();public=root/'public';original=h.read(public/'build-receipt.json')
        for key in ('integration_head','executable_sha256','frozen_commit','base_commit','reconstruction_sha256','episode_patch_sha256','source_manifest_sha256','plan_sha256','effective_inputs_sha256'):
            changed=copy.deepcopy(original);changed[key]='e'*len(changed[key]);h.write(public/'build-receipt.json',changed);self.rehash(public)
            with self.subTest(key=key),self.assertRaises(RuntimeError):h.validate_public(public)
        h.write(public/'build-receipt.json',original);self.rehash(public);h.validate_public(public)
    def test_exposure_without_source_or_build_binding_is_rejected(self):
        root=self.ready();public=root/'public';original=h.read(public/'checks.json')
        for key in ('source_bound','native_build_completed'):
            changed=copy.deepcopy(original);changed[key]=False;h.write(public/'checks.json',changed);self.rehash(public)
            with self.subTest(key=key),self.assertRaises(RuntimeError):h.validate_public(public)
    def test_monotonic_deadline_preserves_no_new_life_margin(self):
        from unittest.mock import patch
        with patch.object(h.time,'monotonic',return_value=100):
            self.assertEqual(h.remaining_time(250,h.NEW_LIFE_MINIMUM_SECONDS),150)
            for deadline in (100,200):
                with self.assertRaises(h.HostedFailure) as error:h.remaining_time(deadline,h.NEW_LIFE_MINIMUM_SECONDS)
                self.assertEqual(error.exception.code,'wall_clock_timeout')

class HostedStudyScopeTests(unittest.TestCase):
    def environment(self):
        return {'GITHUB_EVENT_NAME':'pull_request','GITHUB_RUN_ATTEMPT':'1',
                'TRANSFER_EVENT_ACTION':'opened','GITHUB_REPOSITORY':h.STUDY_REPOSITORY,
                'TRANSFER_HEAD_REPOSITORY':h.STUDY_REPOSITORY,'GITHUB_HEAD_REF':h.STUDY_BRANCH,
                'TRANSFER_STUDY_ID':h.STUDY_ID,'EXPECTED_SOURCE_REVISION':''}
    def test_only_exact_initial_same_repository_study_is_admitted(self):
        from unittest.mock import patch
        import os
        good=self.environment()
        with patch.dict(os.environ,good,clear=True):h.require_initial_study_event()
        for key,bad in [('GITHUB_REPOSITORY','other/A-Life'),('TRANSFER_HEAD_REPOSITORY','fork/A-Life'),
                        ('GITHUB_HEAD_REF','main'),('TRANSFER_STUDY_ID','different-study'),
                        ('TRANSFER_EVENT_ACTION','synchronize'),('GITHUB_RUN_ATTEMPT','2'),
                        ('GITHUB_EVENT_NAME','workflow_dispatch')]:
            with self.subTest(key=key),patch.dict(os.environ,{**good,key:bad},clear=True):
                with self.assertRaises(h.HostedFailure) as error:h.require_initial_study_event()
                self.assertEqual(error.exception.code,'initial_execution_guard')
        with patch.dict(os.environ,good,clear=True),patch.object(h,'file_sha',return_value='e'*64):
            with self.assertRaises(h.HostedFailure):h.require_initial_study_event()
    def test_execute_wrong_branch_or_repository_stops_before_source_or_build(self):
        from unittest.mock import patch
        import os
        for key in ('GITHUB_HEAD_REF','GITHUB_REPOSITORY','TRANSFER_HEAD_REPOSITORY'):
            root=Path(tempfile.mkdtemp(prefix='synthetic-rejected-study-'))
            with self.subTest(key=key),patch.dict(os.environ,{**self.environment(),key:'not-authorized'},clear=True),patch.object(h,'require_space') as space:
                self.assertFalse(h.execute(root,Path.cwd(),'a'*40,'success'))
                space.assert_not_called()
                checks=h.read(root/'public/checks.json')
                self.assertEqual(checks['failure_code'],'initial_execution_guard')
                self.assertFalse(checks['source_bound']);self.assertFalse(checks['exposure_started'])
                self.assertEqual(checks['attempted_cases'],[])

if __name__=='__main__':unittest.main()
