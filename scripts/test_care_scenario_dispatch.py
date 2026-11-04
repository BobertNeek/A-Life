import io
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import tempfile
import unittest
from unittest.mock import patch

import care_scenario_batches as batches
import care_scenario_dispatch as dispatch
from llm_care_scenarios import LEGEND, RANDOM_ROLL


class DispatcherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.ledger, self.run_root = self.base / 'ledger', self.base / 'run'
        self.care, self.evaluator = self.base / 'care', self.base / 'round'
        self.care.write_bytes(b'mocked care executable, never run')
        self.evaluator.write_bytes(b'mocked evaluator, never run')
        self.plan = {'schema': 1, 'scope': batches.SCOPE, 'batches': 2, 'family_count': 2,
                     'levels': 2, 'initial_difficulty': 0, 'decision_budget': 4, 'slots': 1,
                     'trials_per_family': 3, 'pool_per_family': 1, 'holdouts_per_family': 1,
                     'seed_start': 1001, 'sampler_seed': 4, 'excluded_seeds': [1000],
                     'native_evaluator_sha256': batches.file_pin(self.evaluator)['sha256']}
        self.recipe = {'schema': 1, 'legend': LEGEND, 'scenarios': [
            {'id': 'family_%02d' % i, 'map': rows, 'condition': 'newborn_v1',
             'difficulty': i, 'goal': 'nourish_and_survive', 'random_roll': RANDOM_ROLL}
            for i, rows in enumerate((['#####', '#L.F#', '#####'], ['#####', '#F.L#', '#####']))]}
        self.recipe_path = self.base / 'recipe.json'; self.recipe_path.write_text(json.dumps(self.recipe))
        batches.initialize(self.ledger, self.plan)
        self.calls, self.constructions = [], []

    def construct(self, source, founder_seed, binary, evidence, guard):
        # Pure test data. This is not a native constructor or native evidence.
        self.constructions.append((source, founder_seed, binary))
        self.assertEqual(founder_seed, 1000)
        self.assertEqual(binary, self.care)
        source.mkdir()
        for name in dispatch.EVALUATION_SOURCE_FILES | {'compiler-inputs.json'}:
            (source / name).write_bytes(('mock-construction:' + name).encode())
        for name in ('actor-checkpoint.json', 'value-checkpoint.json'):
            (source / name).write_bytes(batches.encode({'optimizer_step': 0}))
        return {'mode': dispatch.CONSTRUCTION_MODE, 'founder_seed': founder_seed,
                'source_sha256': dispatch.evaluation_source_pins(source),
                'world_steps': 0, 'training_rows': 0, 'actor_optimizer_updates': 0,
                'value_optimizer_updates': 0, 'actor_optimizer_step': 0, 'value_optimizer_step': 0,
                'teacher_present': False, 'semantic_prior_enabled': False}

    def native(self, argv, cwd, stdout, stderr):
        self.calls.append(argv)
        self.assertEqual(cwd, self.run_root)
        self.assertEqual(argv[0:3], [str(self.evaluator), 'evaluate', '--spec'])
        request = batches.read_record(Path(argv[3])); output = Path(request['output'])
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'dispatched') if not output.exists() else None
        if not output.exists():
            output.mkdir()
            result = self.make_archive(output, request)
        else:
            result = batches.read_record(output / 'care-evaluation.json')['result']
        stdout.write(batches.encode(result))
        return 0

    def make_archive(self, output, request):
        source = self.run_root / 'initial-source'
        body_before = {'energy': .4, 'health': 1., 'injury': 0.}
        body_after = {'energy': .5, 'health': 1., 'injury': 0.}
        rows = [{'row': i, 'loss_row': False, 'teacher_heard': False,
                 'evaluation_row': i < request['decisions'], 'bootstrap_row': i >= request['decisions'],
                 'organism': 1, 'decision_tick': i, 'outcome_tick': i + 1, 'nourished_meal': i == 0,
                 'outcome': {'measured_physiology': {'before': {'body': body_before},
                                                     'after': {'body': body_after}}}}
                for i in range(request['decisions'] + 1)]
        batches.write_new(output / 'care-trace.json', {'backend': 'native-cpu', 'lesson': request['lesson'],
                                                     'complete': True, 'rows': rows})
        result = {'input_sha256': {}, 'mode': 'native-cpu-care-assessment', 'backend': 'native-cpu',
                  'lesson': request['lesson'], 'seed': request['seed'], 'decisions': request['decisions'],
                  'evaluated_decisions': request['decisions'], 'bootstrap_decisions': 1,
                  'teacher_present': False, 'teacher_cue_frames': 0, 'semantic_prior_enabled': False,
                  'offline_learning_disabled': True, 'offline_actor_optimizer_updates': 0,
                  'offline_value_optimizer_updates': 0, 'actor_checkpoint_unchanged': True,
                  'value_checkpoint_unchanged': True, 'promotion': 'none',
                  'actor_checkpoint_before_sha256': 'a'*64, 'actor_checkpoint_after_sha256': 'a'*64,
                  'value_checkpoint_before_sha256': 'b'*64, 'value_checkpoint_after_sha256': 'b'*64,
                  'policy_version': 1, 'source_sha256': dispatch.evaluation_source_pins(source),
                  'evaluated_nourished_meals': 1}
        native_request = dict(request)
        if native_request['food_need_condition'] == 'newborn_v1':
            del native_request['food_need_condition']  # Native serde omits the default.
        batches.write_new(output / 'care-evaluation.json', {'schema_version': 1, 'request': native_request,
            'input_sha256': {}, 'artifacts': {name: pin['sha256'] for name, pin in dispatch.raw_tree(output).items()},
            'result': result, 'payload_sha256': [0]*32})
        return result

    def initialize(self):
        with patch.object(dispatch, 'construction_only_backend', return_value=self.construct):
            return dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)

    def proposal(self):
        batches.prepare(self.ledger)
        batches.propose(self.ledger, self.recipe_path)
        records = batches.load(self.ledger)
        return {'bank': records[-1]['payload']['bank'], 'discovery': records[-2]['payload']['discovery']}

    def test_two_mock_batches_only_discovery_and_one_zero_update_construction(self):
        self.initialize()
        all_discovery = dispatch.discovery_seeds(self.plan)
        for batch in range(2):
            private = self.proposal()
            with patch.object(dispatch, '_bounded_process', side_effect=self.native):
                result = dispatch.run(self.run_root)
            self.assertEqual(result['batch'], batch)
            self.assertFalse(result['training_enabled'])
            self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'assessed')
            history = batches.read_record(self.run_root / 'batches' / ('%04d' % batch) / 'seed-history.json')
            self.assertEqual(history['consumed_seed_ranges'], [[1000, 1000]])
            self.assertEqual(history['assessment_seeds'], all_discovery)
            dispatched = [batches.read_record(Path(argv[3]))['seed'] for argv in self.calls if argv[1] == 'evaluate']
            self.assertFalse(set(dispatched) & {t['seed'] for k in ('pool', 'holdouts') for t in private['bank'][k]})
            for index in range(6):
                self.assertTrue((self.run_root / 'batches' / ('%04d' % batch) / ('assessment-%04d' % index)).is_dir())
        self.assertEqual(len(self.calls), 2*6*2)  # Each completed archive is admitted once, without another life.
        self.assertEqual(len(self.constructions), 1)
        self.assertFalse(any('--fixture' in call for call in self.calls))
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'new proposal'):
                dispatch.run(self.run_root)
            native.assert_not_called()

    def test_concurrent_run_cannot_duplicate_a_previously_loaded_proposal(self):
        self.initialize(); self.proposal()
        real_claim = batches.claim_dispatch
        inner_results = []
        def competing_worker(ledger, identity):
            # Both workers loaded the same proposed stage. The second gets the
            # durable claim first; the stale first worker must not run a life.
            with patch.object(batches, 'claim_dispatch', side_effect=real_claim):
                inner_results.append(dispatch.run(self.run_root))
            return real_claim(ledger, identity)
        with patch.object(dispatch, '_bounded_process', side_effect=self.native) as native, patch.object(batches, 'claim_dispatch', side_effect=competing_worker):
            with self.assertRaisesRegex(ValueError, 'never replay'): dispatch.run(self.run_root)
            self.assertEqual(native.call_count, 12)
        self.assertEqual(len(inner_results), 1)
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'assessed')

    def test_init_requires_excluded_seed_and_new_separate_root(self):
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'excluded_seeds'):
                dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1009)
            with self.assertRaisesRegex(ValueError, 'separate'):
                dispatch.initialize(self.ledger / 'run', self.ledger, self.care, self.evaluator, 1000)
            with self.assertRaisesRegex(ValueError, 'separate'):
                dispatch.initialize(self.base, self.ledger, self.care, self.evaluator, 1000)
            native.assert_not_called()
        self.initialize()
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'fresh disposable'):
                dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)
            native.assert_not_called()

    def test_same_ledger_cannot_initialize_another_founder(self):
        self.initialize()
        other = self.base / 'another-run'
        with patch.object(dispatch, '_bounded_process') as native, patch.object(dispatch, 'construction_only_backend', return_value=self.construct):
            with self.assertRaises(FileExistsError):
                dispatch.initialize(other, self.ledger, self.care, self.evaluator, 1000)
            native.assert_not_called()
        self.assertFalse(other.exists())

    def test_complete_six_file_source_without_optional_compiler(self):
        def without_compiler(source, founder_seed, binary, evidence, guard):
            result = self.construct(source, founder_seed, binary, evidence, guard)
            (source / 'compiler-inputs.json').unlink()
            result['source_sha256'] = dispatch.evaluation_source_pins(source)
            return result
        with patch.object(dispatch, 'construction_only_backend', return_value=without_compiler):
            dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)
        self.proposal()
        with patch.object(dispatch, '_bounded_process', side_effect=self.native):
            dispatch.run(self.run_root)
        self.assertEqual(len(batches.load(self.ledger)[-2]['payload']['identity']['source_sha256']), 6)

    def test_existing_batch_directory_consumes_claim_without_native_work(self):
        self.initialize(); self.proposal()
        (self.run_root / 'batches' / '0000').mkdir()
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaises(FileExistsError): dispatch.run(self.run_root)
            native.assert_not_called()
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'dispatched')
        with self.assertRaisesRegex(ValueError, 'never replay'): dispatch.run(self.run_root)

    def test_native_verification_mutation_never_commits_feedback(self):
        self.initialize(); self.proposal()
        def mutate_verification(argv, cwd, stdout, stderr):
            output = Path(batches.read_record(Path(argv[3]))['output'])
            verification = output.exists()
            result = self.native(argv, cwd, stdout, stderr)
            if verification:
                (output / 'care-trace.json').write_bytes(b'changed during verification')
            return result
        with patch.object(dispatch, '_bounded_process', side_effect=mutate_verification) as native:
            with self.assertRaisesRegex(ValueError, 'frozen source/archive changed'): dispatch.run(self.run_root)
            self.assertEqual(native.call_count, 7)
        self.assertTrue((self.ledger / '0004').is_dir())
        self.assertFalse((self.ledger / '0004' / 'complete.json').exists())
        with self.assertRaisesRegex(ValueError, 'incomplete stage'): dispatch.run(self.run_root)

    def test_prior_archives_stay_pinned_across_batches(self):
        self.initialize(); self.proposal()
        with patch.object(dispatch, '_bounded_process', side_effect=self.native): dispatch.run(self.run_root)
        self.proposal()
        (self.run_root / 'batches' / '0000' / 'assessment-0000' / 'care-trace.json').write_bytes(b'mutation')
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'completed batch controls or evidence changed'): dispatch.run(self.run_root)
            native.assert_not_called()
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'proposed')

    def test_prior_history_requests_and_command_evidence_remain_pinned(self):
        for target in ('seed-history.json', 'assessment-0000.request.json', 'bank.json',
                       'dispatch-0000/stdout.json', 'verify-0000/command.json'):
            with self.subTest(target=target):
                self.initialize(); self.proposal()
                with patch.object(dispatch, '_bounded_process', side_effect=self.native): dispatch.run(self.run_root)
                self.proposal()
                (self.run_root / 'batches' / '0000' / target).write_bytes(b'changed archived control')
                with patch.object(dispatch, '_bounded_process') as native:
                    with self.assertRaisesRegex(ValueError, 'completed batch controls or evidence changed'):
                        dispatch.run(self.run_root)
                    native.assert_not_called()
                self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'proposed')
                if target != 'verify-0000/command.json':
                    self.temp.cleanup(); self.setUp()

    def test_command_rejects_training_and_arbitrary_evaluation_paths(self):
        self.initialize()
        _, _, _, _, _, guard = dispatch.load_run(self.run_root)
        commands = [(['--fixture', '--output', str(self.run_root / 'initial-source'), '--seed', '1000',
                      '--lesson', 'eat_held_food', '--decisions', '2'], self.care),
                    (['--previous', str(self.run_root / 'initial-source')], self.care),
                    (['train', '--spec', str(self.recipe_path)], self.evaluator),
                    (['evaluate', '--spec', str(self.recipe_path)], self.evaluator)]
        with patch.object(dispatch, '_bounded_process') as native:
            for index, (arguments, binary) in enumerate(commands):
                with self.assertRaises(ValueError):
                    dispatch.command(binary, arguments, self.run_root, self.run_root / ('bad-%d' % index), guard)
            native.assert_not_called()

    def test_constructor_timeout_preserves_unusable_root(self):
        def fail(source, founder_seed, binary, evidence, guard):
            (self.run_root / 'partial-founder').write_bytes(b'uncertain')
            raise subprocess.TimeoutExpired(['mock constructor'], 60)
        with patch.object(dispatch, 'construction_only_backend', return_value=fail):
            with self.assertRaises(subprocess.TimeoutExpired):
                dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)
        self.assertTrue((self.run_root / 'partial-founder').exists())
        self.assertFalse((self.run_root / 'initialized.json').exists())
        with self.assertRaises(FileNotFoundError): dispatch.run(self.run_root)
        with self.assertRaisesRegex(ValueError, 'fresh disposable'):
            dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)

    def test_default_init_is_unavailable_without_any_side_effect(self):
        before = dispatch.tree_pin(self.ledger)
        with patch.object(dispatch, '_bounded_process') as native, patch.object(dispatch.subprocess, 'Popen') as popen, patch.object(batches, 'claim_founder') as claim:
            with self.assertRaises(dispatch.ConstructionOnlyUnavailable):
                dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)
            native.assert_not_called(); popen.assert_not_called(); claim.assert_not_called()
        self.assertFalse(self.run_root.exists())
        self.assertEqual(dispatch.tree_pin(self.ledger), before)

    def test_default_cli_has_no_training_override(self):
        before = dispatch.tree_pin(self.ledger)
        arguments = ['--run-root', str(self.run_root), 'init', '--ledger-root', str(self.ledger),
                     '--care-binary', str(self.care), '--native-evaluator', str(self.evaluator),
                     '--founder-seed', '1000']
        with patch.object(dispatch, '_bounded_process') as native, patch('sys.stderr', new=io.StringIO()) as stderr:
            self.assertEqual(dispatch.main(arguments), 1)
            self.assertIn('construction-only native founder backend', stderr.getvalue())
            for option in ('--fixture', '--allow-training', '--unsafe-training'):
                with self.assertRaises(SystemExit): dispatch.main(arguments + [option])
            native.assert_not_called()
        self.assertFalse(self.run_root.exists())
        self.assertEqual(dispatch.tree_pin(self.ledger), before)

    def test_old_fixture_run_identity_is_not_admitted(self):
        self.initialize(); self.proposal()
        manifest_path = self.run_root / 'run-manifest.json'
        manifest = batches.read_record(manifest_path)
        manifest['schema'] = 1
        del manifest['construction_mode']
        manifest_path.write_bytes(batches.encode(manifest))
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'invalid bound run manifest'): dispatch.run(self.run_root)
            native.assert_not_called()
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'proposed')

    def test_nonzero_constructor_updates_fail_without_initialized_seal(self):
        def trained(source, founder_seed, binary, evidence, guard):
            receipt = self.construct(source, founder_seed, binary, evidence, guard)
            receipt['actor_optimizer_updates'] = 2
            return receipt
        with patch.object(dispatch, 'construction_only_backend', return_value=trained), patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'no world steps or offline updates'):
                dispatch.initialize(self.run_root, self.ledger, self.care, self.evaluator, 1000)
            native.assert_not_called()
        self.assertFalse((self.run_root / 'initialized.json').exists())
        self.assertTrue((self.run_root / 'initial-source').exists())

    def test_dispatch_timeout_retains_claim_and_never_retries(self):
        self.initialize(); self.proposal()
        def fail(argv, cwd, stdout, stderr):
            output = Path(batches.read_record(Path(argv[3]))['output'])
            output.mkdir(); (output / 'private-recovery').write_bytes(b'uncertain life')
            stderr.write(b'native still uncertain')
            raise subprocess.TimeoutExpired(argv, 60)
        with patch.object(dispatch, '_bounded_process', side_effect=fail):
            with self.assertRaises(subprocess.TimeoutExpired): dispatch.run(self.run_root)
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'dispatched')
        batch = self.run_root / 'batches' / '0000'
        self.assertEqual(batches.read_record(batch / 'dispatch-0000' / 'outcome.json')['status'], 'timeout_uncertain')
        self.assertTrue((batch / 'assessment-0000' / 'private-recovery').exists())
        with patch.object(dispatch, '_bounded_process') as native:
            with self.assertRaisesRegex(ValueError, 'never replay'): dispatch.run(self.run_root)
            native.assert_not_called()

    def test_interrupt_retains_evidence_and_dispatch_claim(self):
        self.initialize(); self.proposal()
        with patch.object(dispatch, '_bounded_process', side_effect=KeyboardInterrupt):
            with self.assertRaises(KeyboardInterrupt): dispatch.run(self.run_root)
        self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'dispatched')
        self.assertEqual(batches.read_record(self.run_root / 'batches' / '0000' / 'dispatch-0000' / 'outcome.json')['status'], 'uncertain')

    def test_changed_source_or_native_is_rejected_before_claim(self):
        for target in ('source', 'care', 'evaluator'):
            with self.subTest(target=target):
                self.initialize(); self.proposal()
                path = self.run_root / 'initial-source' / 'phase.txt' if target == 'source' else getattr(self, target)
                original = path.read_bytes(); path.write_bytes(b'mutation')
                with patch.object(dispatch, '_bounded_process') as native:
                    with self.assertRaisesRegex(ValueError, 'changed'): dispatch.run(self.run_root)
                    native.assert_not_called()
                self.assertEqual(batches.load(self.ledger)[-1]['kind'], 'proposed')
                path.write_bytes(original)
                # Restore only mock bytes for an independent case, never native evidence.
                if target != 'evaluator':
                    self.temp.cleanup(); self.setUp()

    def test_post_command_guards_source_binary_bank_and_ledger(self):
        for target in ('source', 'care', 'evaluator', 'bank', 'ledger'):
            with self.subTest(target=target):
                self.initialize(); self.proposal()
                def mutate(argv, cwd, stdout, stderr):
                    result = self.native(argv, cwd, stdout, stderr)
                    path = {'source': self.run_root / 'initial-source' / 'phase.txt', 'care': self.care,
                            'evaluator': self.evaluator, 'bank': self.run_root / 'batches' / '0000' / 'bank.json',
                            'ledger': self.ledger / '0000' / 'record.json'}[target]
                    path.write_bytes(b'changed during native command')
                    return result
                with patch.object(dispatch, '_bounded_process', side_effect=mutate) as native:
                    with self.assertRaisesRegex(ValueError, 'changed'): dispatch.run(self.run_root)
                    self.assertEqual(native.call_count, 1)
                self.assertFalse((self.ledger / '0004').exists())
                if target != 'ledger':
                    self.temp.cleanup(); self.setUp()

    def test_unknown_receipt_fields_fail_before_scoring(self):
        self.initialize(); self.proposal()
        def invalid(argv, cwd, stdout, stderr):
            result = self.native(argv, cwd, stdout, stderr)
            output = Path(batches.read_record(Path(argv[3]))['output'])
            receipt = batches.read_record(output / 'care-evaluation.json')
            receipt['unknown'] = True
            (output / 'care-evaluation.json').write_bytes(batches.encode(receipt))
            return result
        with patch.object(dispatch, '_bounded_process', side_effect=invalid), patch.object(dispatch, '_score_archive_metrics') as score:
            with self.assertRaisesRegex(ValueError, 'unsupported native assessment receipt'): dispatch.run(self.run_root)
            score.assert_not_called()
        with self.assertRaisesRegex(ValueError, 'incomplete stage'): batches.load(self.ledger)

    def test_no_accepted_source_or_implicit_seed_cli(self):
        with patch.object(dispatch, '_bounded_process') as native, patch('sys.stderr', new=io.StringIO()):
            for extra in (['--previous', '/retained-creature'], []):
                with self.assertRaises(SystemExit):
                    dispatch.main(['--run-root', str(self.run_root), 'init', '--ledger-root', str(self.ledger),
                                   '--care-binary', str(self.care), '--native-evaluator', str(self.evaluator), *extra])
            native.assert_not_called()


class FakeProcess:
    def __init__(self, stdout=b'{}\n', stderr=b'', *, keep_open=False):
        self.pid, self.writers = 424242, []
        for name, contents in (('stdout', stdout), ('stderr', stderr)):
            read, write = os.pipe()
            os.write(write, contents)
            if keep_open:
                self.writers.append(write)
            else:
                os.close(write)
            setattr(self, name, os.fdopen(read, 'rb'))

    def wait(self, timeout):
        for descriptor in self.writers: os.close(descriptor)
        self.writers = []
        return 0


class BoundedProcessTests(unittest.TestCase):
    def test_no_shell_or_credentials_and_fixed_prior_off(self):
        fake = FakeProcess(b'{"ok":true}\n', b'native log')
        with patch.object(dispatch.subprocess, 'Popen', return_value=fake) as popen, patch.dict(os.environ, {'SECRET_API_KEY': 'private', 'LD_PRELOAD': '/bad.so', 'ALIFE_SLM_PRIOR': 'on'}):
            stdout, stderr = io.BytesIO(), io.BytesIO()
            self.assertEqual(dispatch._bounded_process(['/pinned/native', 'evaluate', '--spec', '/fixed/spec'], Path('/tmp'), stdout, stderr), 0)
        kwargs = popen.call_args.kwargs
        self.assertIs(kwargs['shell'], False)
        self.assertTrue(kwargs['start_new_session'])
        self.assertEqual(kwargs['stdin'], subprocess.DEVNULL)
        self.assertNotIn('SECRET_API_KEY', kwargs['env']); self.assertNotIn('LD_PRELOAD', kwargs['env'])
        self.assertEqual(kwargs['env']['ALIFE_SLM_PRIOR'], 'off')
        self.assertEqual(stdout.getvalue(), b'{"ok":true}\n')
        self.assertEqual(stderr.getvalue(), b'native log')

    def test_combined_output_bound_kills_and_retains_prefix(self):
        fake = FakeProcess(b'x'*20, b'y'*20)
        with patch.object(dispatch.subprocess, 'Popen', return_value=fake), patch.object(dispatch, 'MAX_COMMAND_BYTES', 16), patch.object(dispatch.os, 'killpg') as kill:
            stdout, stderr = io.BytesIO(), io.BytesIO()
            with self.assertRaisesRegex(ValueError, 'output exceeds bound'):
                dispatch._bounded_process(['/pinned/native'], Path('/tmp'), stdout, stderr)
            self.assertEqual(len(stdout.getvalue()) + len(stderr.getvalue()), 16)
            kill.assert_called_once_with(fake.pid, dispatch.signal.SIGKILL)

    def test_term_and_hup_kill_reap_child_and_restore_handlers(self):
        # Harmless Python sleeper only: no native care program or biological life.
        for signum in (dispatch.signal.SIGTERM, dispatch.signal.SIGHUP):
            with self.subTest(signal=signum), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                pidfile, resultfile = root / 'child.pid', root / 'result.json'
                child = "import os,pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text(str(os.getpid())); time.sleep(30)"
                wrapper = "\n".join([
                    "import json, pathlib, signal, sys",
                    "sys.path.insert(0, %r)" % str(Path(dispatch.__file__).parent),
                    "import care_scenario_dispatch as dispatch",
                    "root = pathlib.Path(%r)" % str(root),
                    "previous = {s: signal.getsignal(s) for s in (signal.SIGTERM, signal.SIGHUP)}",
                    "try:",
                    "    with (root / 'stdout').open('wb') as out, (root / 'stderr').open('wb') as err:",
                    "        dispatch._bounded_process([sys.executable, '-c', %r, %r], root, out, err)" % (child, str(pidfile)),
                    "except dispatch.NativeCommandTerminated as error:",
                    "    (root / 'result.json').write_text(json.dumps({'signal': error.signum, 'restored': all(signal.getsignal(s) == handler for s, handler in previous.items())}))",
                ])
                worker = subprocess.Popen([sys.executable, '-c', wrapper], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                child_pid = None
                try:
                    deadline = time.monotonic() + 5
                    while not pidfile.exists() and time.monotonic() < deadline:
                        if worker.poll() is not None:
                            break
                        time.sleep(.01)
                    self.assertTrue(pidfile.exists(), worker.poll())
                    child_pid = int(pidfile.read_text())
                    os.kill(worker.pid, signum)
                    stdout, stderr = worker.communicate(timeout=5)
                    self.assertEqual(worker.returncode, 0, stderr.decode())
                    self.assertEqual(json.loads(resultfile.read_text()), {'signal': signum, 'restored': True})
                    with self.assertRaises(ProcessLookupError): os.kill(child_pid, 0)
                finally:
                    if worker.poll() is None:
                        worker.kill(); worker.wait(timeout=5)
                    if child_pid is not None:
                        try: os.killpg(child_pid, dispatch.signal.SIGKILL)
                        except ProcessLookupError: pass
                    worker.stdout.close(); worker.stderr.close()

    def test_timeout_kills_group_without_retry(self):
        fake = FakeProcess(keep_open=True)
        with patch.object(dispatch.subprocess, 'Popen', return_value=fake) as popen, patch.object(dispatch, 'COMMAND_TIMEOUT', 0.001), patch.object(dispatch.os, 'killpg') as kill:
            with self.assertRaises(subprocess.TimeoutExpired):
                dispatch._bounded_process(['/pinned/native'], Path('/tmp'), io.BytesIO(), io.BytesIO())
            self.assertEqual(popen.call_count, 1)
            kill.assert_called_once_with(fake.pid, dispatch.signal.SIGKILL)


if __name__ == '__main__':
    unittest.main()
