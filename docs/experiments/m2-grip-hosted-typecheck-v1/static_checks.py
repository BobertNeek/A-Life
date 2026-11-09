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
        self.marker = {'schema': 'm2-grip-typecheck-activation-v1', 'plan_id': r.PLAN,
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
        self.assertEqual(m['required_prepared_workflows'], ['.github/workflows/m2-grip-hosted-typecheck-v1.yml'])
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
        self.assertIn('paths: [docs/experiments/m2-grip-hosted-typecheck-v1/ACTIVATE.json]', workflow)
        for forbidden in ['workflow_dispatch:', 'pull_request:', 'secrets.', 'upload-artifact', 'actions/cache', 'contents: write']:
            self.assertNotIn(forbidden, workflow)
        self.assertEqual(workflow.count('    runs-on:'), 1)

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
    summary = {'schema': 'm2-grip-static-check-v1', 'checks_run': result.testsRun,
               'passed': result.wasSuccessful(), 'failures': len(result.failures), 'errors': len(result.errors),
               'native_invocations': 0, 'network_calls': 0, 'test_bodies': 0,
               'scope': 'Python syntax, pure synthetic guards/projections, source recovery and metadata only'}
    (ROOT / 'STATIC-CHECK.json').write_text(json.dumps(summary, indent=2, sort_keys=True) + '\n')
    raise SystemExit(0 if result.wasSuccessful() else 1)
