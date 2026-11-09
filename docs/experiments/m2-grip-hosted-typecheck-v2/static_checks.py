#!/usr/bin/env python3
"""Only Python synthetic/static checks; no native child process is invoked."""
import ast
import copy
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest.mock import patch

ROOT = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('runner', ROOT / 'run_hosted.py')
r = importlib.util.module_from_spec(spec); spec.loader.exec_module(r)
spec = importlib.util.spec_from_file_location('source_patch', ROOT / 'source_patch.py')
p = importlib.util.module_from_spec(spec); spec.loader.exec_module(p)

class StaticTests(unittest.TestCase):
    def setUp(self):
        self.before = 'a' * 40
        self.head = 'b' * 40
        self.env = {'GITHUB_ACTIONS': 'true', 'GITHUB_EVENT_NAME': 'push',
            'GITHUB_REPOSITORY': 'BobertNeek/A-Life', 'GITHUB_REF': r.BRANCH,
            'GITHUB_RUN_ATTEMPT': '1'}
        self.event = {'before': self.before, 'after': self.head, 'ref': r.BRANCH,
                      'created': False, 'deleted': False, 'forced': False}
        self.marker = {'schema': 'm2-grip-typecheck-activation-v2', 'plan_id': r.PLAN,
            'main': r.MAIN, 'research_base': r.FBF, 'prepared_head': self.before,
            'seal_sha256': 'c' * 64, 'source_manifest_sha256': 'd' * 64,
            'attempt': 1, 'typecheck_only': True}
        self.args = [self.env, self.event, self.head, [self.head, self.before],
                     'A\t' + r.MARKER + '\n', self.marker, 'c' * 64, 'd' * 64]

    def test_exact_activation(self):
        self.assertEqual(r.validate_activation(*self.args), self.before)

    def test_wrong_environment(self):
        for key in self.env:
            args = copy.deepcopy(self.args); args[0][key] = 'invalid'
            with self.subTest(key=key), self.assertRaises(r.Stop):
                r.validate_activation(*args)

    def test_event_guards(self):
        for key, value in [('created', True), ('deleted', True), ('forced', True),
                           ('before', '0' * 40), ('after', 'f' * 40), ('ref', 'refs/heads/main')]:
            args = copy.deepcopy(self.args); args[1][key] = value
            with self.subTest(key=key), self.assertRaises(r.Stop):
                r.validate_activation(*args)

    def test_marker_must_be_only_new_path(self):
        for changed in ['M\t' + r.MARKER + '\n', 'A\t' + r.MARKER + '\nM\tCargo.toml\n', '', 'A\told/ACTIVATE.json\n']:
            args = copy.deepcopy(self.args); args[4] = changed
            with self.subTest(changed=changed), self.assertRaises(r.Stop):
                r.validate_activation(*args)

    def test_marker_is_exact(self):
        for key in self.marker:
            args = copy.deepcopy(self.args); args[5][key] = 'invalid'
            with self.subTest(key=key), self.assertRaises(r.Stop):
                r.validate_activation(*args)
        args = copy.deepcopy(self.args); args[5]['extra'] = 'not allowed'
        with self.assertRaises(r.Stop): r.validate_activation(*args)

    def test_parent_cannot_be_merge_or_stale(self):
        for parents in [[], [self.head], [self.head, 'f' * 40], [self.head, self.before, 'e' * 40]]:
            args = copy.deepcopy(self.args); args[3] = parents
            with self.assertRaises(r.Stop): r.validate_activation(*args)

    def test_diagnostics_strict_projection(self):
        source = pathlib.Path('/safe/source')
        name = 'crates/alife_core/src/lib.rs'
        obj = {'message': {'level': 'error', 'code': {'code': 'E0308', 'explanation': 'secret'},
              'message': 'private text', 'rendered': 'private rendering',
              'spans': [{'is_primary': True, 'file_name': str(source / name),
                         'line_start': 12, 'text': ['private snippet'], 'column_start': 5}]}}
        self.assertEqual(r.diagnostic_rows(obj, 'core', source, {name: {}}),
                         [{'code': 'E0308', 'path': name, 'line': 12, 'stage': 'core'}])

    def test_diagnostics_block_unknown_paths_and_codes(self):
        source = pathlib.Path('/safe/source'); name = 'crates/alife_core/src/lib.rs'
        base = {'message': {'level': 'error', 'code': {'code': 'E0308'},
            'spans': [{'is_primary': True, 'file_name': name, 'line_start': 1}]}}
        for bad in ['../../private', '/private/token', 'docs/private.rs', 'crates/alife_core/src/../secret']:
            obj = copy.deepcopy(base); obj['message']['spans'][0]['file_name'] = bad
            self.assertEqual(r.diagnostic_rows(obj, 'core', source, {name: {}}), [])
        for bad in [None, {'code': 'private'}, {'code': 'E0001\nsecret'}]:
            obj = copy.deepcopy(base); obj['message']['code'] = bad
            self.assertEqual(r.diagnostic_rows(obj, 'core', source, {name: {}}), [])
        self.assertEqual(r.diagnostic_rows(base, 'core', source, {}), [])

    def test_diagnostics_line_and_primary(self):
        name = 'crates/alife_core/src/lib.rs'
        for line in [True, -1, 0, 10000001, 'secret']:
            obj = {'message': {'level': 'error', 'code': {'code': 'E0308'},
                'spans': [{'is_primary': True, 'file_name': name, 'line_start': line}]}}
            self.assertEqual(r.diagnostic_rows(obj, 'core', pathlib.Path('/s'), {name: {}}), [])

    def test_text_patch_new_and_existing(self):
        data = b'--- a/crates/a.rs\n+++ b/crates/a.rs\n@@ -1 +1 @@\n-old\n+new\n--- /dev/null\n+++ b/crates/b.rs\n@@ -0,0 +1 @@\n+added\n'
        self.assertEqual(p.apply(data, lambda x: b'old\n' if x.endswith('a.rs') else None),
                         {'crates/a.rs': b'new\n', 'crates/b.rs': b'added\n'})

    def test_patch_rejects_mismatch_binary_traversal(self):
        samples = [b'GIT binary patch\n',
            b'--- a/a.rs\n+++ b/../a.rs\n@@ -1 +1 @@\n-old\n+new\n',
            b'--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-wrong\n+new\n']
        for sample in samples:
            with self.assertRaises(AssertionError): p.apply(sample, lambda x: b'old\n')

    def test_nominal_target_feature_scope(self):
        self.assertEqual(len(r.STAGES), 6)
        training = next(x for x in r.STAGES if x[1] == 'alife_training')
        self.assertEqual(training[2], ('--lib', '--test', 'grip_options_host'))
        self.assertEqual(training[3], ('cpu-backend',))
        self.assertEqual(r.STAGES[-1][3], ('cpu-training',))
        for row in r.STAGES:
            self.assertNotIn('--no-default-features', row[2])
            self.assertNotIn('--all-features', row[2])

    def test_source_and_recovery_metadata(self):
        m = json.loads((ROOT / 'source-manifest.json').read_bytes())
        self.assertEqual(len(m['files']), 864)
        self.assertEqual(m['unresolved_includes'], [])
        self.assertEqual(len(set(x['path'] for x in m['files'])), 864)
        self.assertIn('crates/alife_training/tests/grip_options_host.rs', [x['path'] for x in m['files']])
        self.assertNotIn('crates/alife_training/tests/fixtures/cpu-resident-pre-motor-v1.json.zst', [x['path'] for x in m['files']])
        recovery = json.loads((ROOT / 'recovery.json').read_bytes())
        for step in recovery['steps']:
            data = b''.join((ROOT / row['path']).read_bytes() for row in step['parts'])
            self.assertEqual(hashlib.sha256(data).hexdigest(), step['sha256'])
            self.assertEqual(len(data), step['bytes'])

    def test_publication_has_no_old_workflows(self):
        m = json.loads((ROOT / 'publication.json').read_bytes())
        self.assertEqual(len(m['branch_local_deletions']), 15)
        self.assertEqual(m['required_prepared_workflows'], ['.github/workflows/m2-grip-hosted-typecheck-v2.yml'])
        self.assertEqual(m['remote_actions_performed'], [])
        self.assertIn('PENDING', m['prepared_head'])

    def test_python_ast_and_no_unbounded_native_execution(self):
        for file in ROOT.glob('*.py'):
            ast.parse(file.read_text(), filename=file.name)
        source = (ROOT / 'run_hosted.py').read_text()
        self.assertNotIn("'cargo', 'test'", source)
        self.assertNotIn("'cargo', 'run'", source)
        self.assertNotIn('--no-default-features', source)
        self.assertIn("'cargo', 'check', '--locked', '-j1'", source)
        self.assertIn("'CARGO_BUILD_JOBS': '1'", source)
        self.assertIn("'CARGO_NET_RETRY': '0'", source)
        self.assertIn('start_new_session=True', source)
        self.assertIn('verify_depinfo(artifacts)', source)

    def test_workflow_is_single_readonly_marker_job(self):
        workflow = (ROOT / 'workflow.yml').read_text()
        self.assertIn('EXPECTED_SEAL_SHA256: ', workflow)
        self.assertIn('timeout-minutes: 90', workflow)
        self.assertEqual(workflow.count('persist-credentials: false'), 2)
        self.assertIn('paths: [docs/experiments/m2-grip-hosted-typecheck-v2/ACTIVATE.json]', workflow)
        for forbidden in ['workflow_dispatch:', 'pull_request:', 'secrets.', 'upload-artifact', 'actions/cache', 'contents: write']:
            self.assertNotIn(forbidden, workflow)
        self.assertEqual(workflow.count('    runs-on:'), 1)

    def test_runner_semantics_unchanged_except_version_and_input(self):
        old = (ROOT.parent / 'hosted-typecheck-v1/run_hosted.py').read_text()
        for a, b in [('m2-grip-hosted-typecheck-v1', 'm2-grip-hosted-typecheck-v2'),
                     ('m2-grip-typecheck-activation-v1', 'm2-grip-typecheck-activation-v2'),
                     ('m2-grip-typecheck-summary-v1', 'm2-grip-typecheck-summary-v2')]:
            old = old.replace(a, b)
        old = old.replace("STATE.update(main=MAIN, research_base=FBF, grip_patch_sha256=recovery['steps'][-1]['sha256'],",
                          "STATE.update(main=MAIN, research_base=FBF, grip_patch_sha256=recovery['steps'][-2]['sha256'],\n                 correction_patch_sha256=recovery['steps'][-1]['sha256'],")
        self.assertEqual((ROOT / 'run_hosted.py').read_text(), old)

    def test_v1_preserved_and_reused_blobs_exact(self):
        prior = ROOT.parent / 'hosted-typecheck-v1'
        evidence = json.loads((ROOT / 'V1-PRESERVATION.json').read_bytes())
        for row in evidence['files']:
            data = (prior / row['path']).read_bytes()
            self.assertEqual(len(data), row['bytes'])
            self.assertEqual(hashlib.sha256(data).hexdigest(), row['sha256'])
        reuse = json.loads((ROOT / 'REUSE-MANIFEST.json').read_bytes())
        self.assertEqual(reuse['verified_source_commit'], '4a3a39be5f8a4a95f662ed2e7c3a67ed50b62da1')
        for row in reuse['files']:
            data = (ROOT / row['path']).read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(), row['sha256'])
            self.assertEqual(hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest(), row['git_blob_sha1'])
            self.assertEqual(data, (prior / row['path']).read_bytes())
            self.assertNotIn(row['path'], ['workflow.yml', 'ACTIVATE.json', 'run_hosted.py'])

    def test_correction_and_source_delta(self):
        delta = json.loads((ROOT / 'SOURCE-DELTA.json').read_bytes())
        self.assertEqual(len(delta['changed_files']), 3)
        self.assertEqual(delta['correction_patch']['sha256'], '0ce2a14320e13e31b9727d41d979798910c31e9663dcaca043c0900c5c96774a')
        source = json.loads((ROOT / 'source-manifest.json').read_bytes())
        before = json.loads((ROOT.parent / 'hosted-typecheck-v1/source-manifest.json').read_bytes())
        self.assertEqual(sum(a != b for a, b in zip(before['files'], source['files'])), 3)
        self.assertEqual(source['stages'], before['stages'])
        self.assertEqual(source['includes'], before['includes'])
        self.assertEqual(source['registered_source_closure'], '434abe6295edbb7badfc2bc71b8876ba8cee3dd91e87478f46f3ce408850b150')
        self.assertEqual(len(json.loads((ROOT / 'registered-source-v5.json').read_bytes())['files']), 147)

    def test_first_result_is_safe_and_unchanged(self):
        provenance = json.loads((ROOT / 'FIRST-RESULT-PROVENANCE.json').read_bytes())
        data = (ROOT / 'FIRST-RESULT-v1.json').read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), provenance['safe_summary_sha256'])
        result = json.loads(data)
        self.assertEqual(result['completed'], ['core'])
        self.assertEqual(result['stage'], 'world')
        self.assertEqual(result['diagnostics'], [{'code': 'E0004', 'line': 201, 'path': 'crates/alife_world/tests/gpu_memory_grounding_persistence.rs', 'stage': 'world'}])
        self.assertEqual(result['elapsed_seconds'], 78)
        for key in ('native_test_bodies', 'source_forwards', 'world_commands', 'body_advances'):
            self.assertEqual(result[key], 0)

    def test_limit_constants(self):
        self.assertEqual(r.LIMITS['wall_seconds'], 5400)
        self.assertEqual(r.LIMITS['rss_bytes'], 6 * 1024**3)
        self.assertEqual(r.LIMITS['workspace_bytes'], 12 * 1024**3)
        self.assertEqual(r.LIMITS['free_bytes'], 2 * 1024**3)
        self.assertEqual(r.LIMITS['summary_bytes'], 32768)

    def test_depinfo_has_only_bound_inputs(self):
        with tempfile.TemporaryDirectory(dir=ROOT) as d:
            root = pathlib.Path(d); source = root / 'source'; work = root / 'work'
            (source / 'crates/alife_core/src').mkdir(parents=True)
            (work / 'target/debug/deps').mkdir(parents=True)
            name = 'crates/alife_core/src/lib.rs'; (source / name).write_bytes(b'pub fn f() {}\n')
            row = {'path': name, 'bytes': 14, 'sha256': hashlib.sha256(b'pub fn f() {}\n').hexdigest()}
            row['bytes'] = (source / name).stat().st_size
            dep = work / 'target/debug/deps/alife_core-a.d'
            dep.write_text(str(work / 'target/libalife_core-a.rmeta') + ': ' + str(source / name) + '\n')
            artifact = {'target': {'src_path': str(source / name)}, 'filenames': [str(work / 'target/libalife_core-a.rmeta')]}
            with patch.object(r, 'SOURCE', source), patch.object(r, 'WORK', work), patch.object(r, 'BOUND', {name: row}):
                r.verify_depinfo([artifact])
                dep.write_text(str(work / 'target/libalife_core-a.rmeta') + ': ' + str(source / name) + ' ' + str(source / 'private.bin') + '\n')
                with self.assertRaises(r.Stop): r.verify_depinfo([artifact])

    def test_summary_does_not_include_external_exception(self):
        self.assertNotIn('private', r.summary('INTERNAL'))
        self.assertLessEqual(len(r.summary('INTERNAL').encode()), 32768)

    def test_session_rss_sample(self):
        self.assertGreater(r.session_rss(-1), 0)

if __name__ == '__main__':
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(StaticTests)
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    old_path = ROOT.parent / 'hosted-typecheck-v1/run_hosted.py'
    old_tree = ast.parse(old_path.read_text())
    new_tree = ast.parse((ROOT / 'run_hosted.py').read_text())
    def assignment(tree, name):
        return next(ast.dump(node.value, include_attributes=False) for node in tree.body
                    if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets))
    unchanged_constants = {name: assignment(old_tree, name) == assignment(new_tree, name)
                           for name in ['STAGES', 'EXPECTED_FEATURES', 'LIMITS']}
    old_functions = {n.name: ast.dump(n, include_attributes=False) for n in old_tree.body if isinstance(n, ast.FunctionDef)}
    new_functions = {n.name: ast.dump(n, include_attributes=False) for n in new_tree.body if isinstance(n, ast.FunctionDef)}
    normalized_old_functions = {name: body.replace('m2-grip-hosted-typecheck-v1', 'm2-grip-hosted-typecheck-v2').replace('m2-grip-typecheck-activation-v1', 'm2-grip-typecheck-activation-v2').replace('m2-grip-typecheck-summary-v1', 'm2-grip-typecheck-summary-v2') for name, body in old_functions.items()}
    differing_functions = sorted(name for name in new_functions if normalized_old_functions[name] != new_functions[name])
    assert all(unchanged_constants.values()) and differing_functions == ['restore_sources']
    semantic_report = {'schema': 'm2-grip-runner-preservation-v2',
        'v1_runner_sha256': hashlib.sha256(old_path.read_bytes()).hexdigest(),
        'v2_runner_sha256': hashlib.sha256((ROOT / 'run_hosted.py').read_bytes()).hexdigest(),
        'whole_file_normalized_exact_match': result.wasSuccessful(),
        'unchanged_constants': unchanged_constants,
        'functions_differing_after_version_normalization': differing_functions,
        'unchanged_functions_after_version_normalization': sorted(name for name in new_functions if normalized_old_functions[name] == new_functions[name]),
        'allowed_changes': [
            'Literal m2-grip-hosted-typecheck-v1 identifiers become m2-grip-hosted-typecheck-v2 for plan, branch, packet, marker path and sole workflow path.',
            'Activation schema changes from m2-grip-typecheck-activation-v1 to m2-grip-typecheck-activation-v2.',
            'Summary schema changes from m2-grip-typecheck-summary-v1 to m2-grip-typecheck-summary-v2.',
            'restore_sources reports the unchanged full V4 grip hash from recovery.steps[-2] rather than [-1], because the new correction is appended.',
            'restore_sources additionally reports correction_patch_sha256 from recovery.steps[-1].'],
        'not_changed': ['stages', 'targets', 'features', 'serial execution', 'toolchain setup', 'limits', 'resource accounting', 'diagnostic projection', 'admission checks', 'dependency/source verification', 'output bounds', 'no test/body execution'],
        'native_invocations': 0}
    (ROOT / 'RUNNER-SEMANTICS.json').write_text(json.dumps(semantic_report, indent=2, sort_keys=True) + '\n')
    summary = {'schema': 'm2-grip-static-check-v2', 'checks_run': result.testsRun,
               'passed': result.wasSuccessful(), 'failures': len(result.failures), 'errors': len(result.errors),
               'native_invocations': 0, 'network_calls': 0, 'test_bodies': 0,
               'scope': 'Python syntax, pure synthetic guards/projections, source recovery and metadata only'}
    (ROOT / 'STATIC-CHECK.json').write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n')
    raise SystemExit(0 if result.wasSuccessful() else 1)
