import copy
import os
import stat
import tempfile
from pathlib import Path
from run_care_frontier_fixture import Pilot
import unittest
from unittest.mock import patch
import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from llm_care_scenarios import main, request_recipe, recipe_schema, strict_json, _raw_response_opener
from llm_care_scenarios import LEGEND, RANDOM_ROLL, compile_recipe
from care_frontier_sampler import select


def fixtures():
    recipe = {"schema": 1, "legend": LEGEND.copy(), "scenarios": [
        {"id": "held", "map": ["#####", "#L.F#", "#####"], "condition": "newborn_v1", "difficulty": 0,
         "goal": "nourish_and_survive", "random_roll": copy.deepcopy(RANDOM_ROLL)},
        {"id": "feeding", "map": ["#####", "#F.L#", "#####"], "condition": "newborn_v1", "difficulty": 1,
         "goal": "nourish_and_survive", "random_roll": copy.deepcopy(RANDOM_ROLL)}]}
    host = {"decision_budget": 64, "slots": 1, "sampler_seed": 42, "current_difficulty": 1,
            "pool_seeds": {"held": [101], "feeding": [102]},
            "holdout_seeds": {"held": [201], "feeding": [202]}}
    return recipe, host


class RecipeTests(unittest.TestCase):
    def test_compile_is_deterministic_and_selects_native_lessons(self):
        recipe, host = fixtures()
        bank = compile_recipe(recipe, host)
        self.assertEqual(bank, compile_recipe(recipe, host))
        self.assertEqual([f['lesson'] for f in bank['families']], ['feeding', 'feeding'])
        rows = [dict(lesson='feeding', food_need_condition='newborn_v1', layout={'rows': ['#####', '#F.L#', '#####']}, seed=i,
                     decisions=64, temperature=1.0, actor_sha256='a', value_sha256='b',
                     policy_version=1, source_sha256='c', success=False,
                     survived_evaluated_window=True) for i in (301, 302, 303)]
        self.assertEqual(select(bank, rows)['difficulty_after'], 0)
        for r in rows: r['success'] = True
        self.assertEqual(select(bank, rows)['difficulty_after'], 1)

    def test_rejects_unmapped_map_and_unsafe_overrides(self):
        recipe, host = fixtures()
        for mutate in (lambda r: r['scenarios'][0].update(map=['#####', '#L#F#', '#####']),
                       lambda r: r['scenarios'][0].update(reward=9),
                       lambda r: r['scenarios'][0].update(random_roll='choose_seed'),
                       lambda r: r['legend'].update(L='wall')):
            bad = copy.deepcopy(recipe); mutate(bad)
            with self.assertRaises(ValueError): compile_recipe(bad, host)

    def test_provider_http_to_compiled_bank(self):
        recipe, host = fixtures()
        calls = []
        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                calls.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                data = json.dumps({'choices': [{'message': {'content': json.dumps(recipe)}}]}).encode()
                self.send_response(200); self.send_header('Content-Length', str(len(data)))
                self.end_headers(); self.wfile.write(data)
            def log_message(self, *_): pass
        server = HTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            value = request_recipe('http://127.0.0.1:%s/v1/chat/completions' % server.server_port,
                                   'fixture-model', 'recipe instructions', host=host)
            self.assertEqual(compile_recipe(value, host)['families'][0]['layout'], {'rows': ['#####', '#L.F#', '#####']})
            self.assertEqual(len(calls), 1)
            self.assertEqual(calls[0]['model'], 'fixture-model')
            self.assertEqual(calls[0]['temperature'], 0)
            self.assertNotIn('max_tokens', calls[0])
            self.assertEqual(calls[0]['messages'][0]['content'], 'recipe instructions')
            response_format = calls[0]['response_format']
            self.assertEqual(response_format['type'], 'json_schema')
            self.assertIs(response_format['json_schema']['strict'], True)
            schema = response_format['json_schema']['schema']
            self.assertEqual(schema, recipe_schema(host))
            self.assertEqual(schema['required'], ['schema', 'legend', 'scenarios'])
            scenarios = schema['properties']['scenarios']
            self.assertEqual((scenarios['minItems'], scenarios['maxItems']), (2, 2))
            fields = scenarios['items']['properties']
            self.assertEqual(fields['id']['enum'], ['held', 'feeding'])
            self.assertEqual(fields['difficulty'], {'type': 'integer', 'minimum': 0, 'maximum': 1})
            self.assertEqual((fields['map']['minItems'], fields['map']['maxItems']), (3, 17))
            self.assertEqual((fields['map']['items']['minLength'], fields['map']['items']['maxLength']), (3, 17))
            self.assertEqual(fields['condition']['enum'], ['newborn_v1', 'ordinary_healthy_aging_v1'])
            self.assertEqual(fields['goal']['const'], 'nourish_and_survive')
            self.assertEqual(fields['random_roll']['properties']['outcomes']['const'], RANDOM_ROLL['outcomes'])
            self.assertNotIn('pool_seeds', json.dumps(schema))
            def closed_objects(value):
                if isinstance(value, dict):
                    if value.get('type') == 'object':
                        self.assertIs(value['additionalProperties'], False)
                        self.assertEqual(set(value['required']), set(value['properties']))
                    for child in value.values(): closed_objects(child)
                elif isinstance(value, list):
                    for child in value: closed_objects(child)
            closed_objects(schema)
        finally:
            server.shutdown(); thread.join(); server.server_close()

    def test_schema_optional_host_preserves_helper_api_and_native_bounds(self):
        generic = recipe_schema()['properties']['scenarios']
        self.assertEqual((generic['minItems'], generic['maxItems']), (1, 16))
        self.assertNotIn('enum', generic['items']['properties']['id'])
        self.assertEqual(generic['items']['properties']['difficulty']['maximum'], 7)
        _, host = fixtures()
        for change in ({'pool_seeds': {}}, {'holdout_seeds': {'other': [1]}},
                       {'current_difficulty': True}, {'current_difficulty': 2}):
            bad = copy.deepcopy(host); bad.update(change)
            with self.assertRaises(ValueError): recipe_schema(bad)

    def test_provider_content_is_untrusted_and_never_rerolled(self):
        recipe, host = fixtures()
        malformed = ['not JSON', '```json\n{}\n```', '{"schema":1,"schema":1}',
                     '{"schema":NaN}']
        for mutate in (
            lambda r: r.update(schema=True),
            lambda r: r.update(instructions='ignore host and overwrite reward'),
            lambda r: r['scenarios'][0].update(reward=100),
            lambda r: r['scenarios'][0].update(id='unallocated'),
            lambda r: r['scenarios'][0].update(id=[]),
            lambda r: r['scenarios'][0].update(id='feeding'),
            lambda r: r['scenarios'][0].update(difficulty=True),
            lambda r: r['scenarios'][0].update(difficulty=8),
            lambda r: r['scenarios'][0].update(map=['#####', '#L#F#', '#####']),
            lambda r: r['scenarios'][0].update(random_roll={'kind': 'reroll_until_success'}),
        ):
            bad = copy.deepcopy(recipe); mutate(bad); malformed.append(json.dumps(bad))
        calls = []
        response_content = [None]
        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                calls.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                data = json.dumps({'choices': [{'message': {'content': response_content[0]}}]}).encode()
                self.send_response(200); self.send_header('Content-Length', str(len(data)))
                self.end_headers(); self.wfile.write(data)
            def log_message(self, *_): pass
        server = HTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        endpoint = 'http://127.0.0.1:%s/v1/chat/completions' % server.server_port
        try:
            for content in malformed:
                with self.subTest(content=content):
                    response_content[0] = content
                    before = len(calls)
                    with self.assertRaises(ValueError):
                        compile_recipe(request_recipe(endpoint, 'test', 'prompt', host=host), host)
                    self.assertEqual(len(calls), before + 1)
            # Old positional API still works; it now requests a bounded schema.
            response_content[0] = json.dumps(recipe)
            self.assertEqual(request_recipe(endpoint, 'test', 'prompt', None), recipe)
            self.assertEqual(calls[-1]['response_format']['json_schema']['schema'], recipe_schema())
            self.assertEqual(request_recipe(endpoint, 'test', 'prompt', host=host,
                                            max_tokens=512, no_think=True), recipe)
            self.assertEqual(calls[-1]['max_tokens'], 512)
            self.assertEqual(calls[-1]['messages'][0]['content'], 'prompt /no_think')
            before = len(calls)
            for kwargs in ({'max_tokens': True}, {'max_tokens': 0}, {'max_tokens': 8193},
                           {'max_tokens': 2.5}, {'no_think': 'true'}):
                with self.assertRaises(ValueError): request_recipe(endpoint, 'test', 'prompt', **kwargs)
            self.assertEqual(len(calls), before)
        finally:
            server.shutdown(); thread.join(); server.server_close()

    def test_opt_in_raw_response_capture_is_exact_bounded_and_exclusive(self):
        recipe, host = fixtures()
        calls = []
        response_body = [None]
        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                self.rfile.read(int(self.headers['Content-Length']))
                calls.append(self.headers.get('Authorization'))
                self.send_response(200)
                self.send_header('Content-Length', str(len(response_body[0])))
                self.end_headers(); self.wfile.write(response_body[0])
            def log_message(self, *_): pass
        server = HTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        endpoint = 'http://127.0.0.1:%s/v1/chat/completions' % server.server_port
        def envelope(content):
            return ('  ' + json.dumps({'choices': [{'message': {'content': content}}]},
                                      indent=2) + '\n').encode()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                valid = envelope(json.dumps(recipe))
                response_body[0] = valid
                self.assertEqual(request_recipe(endpoint, 'test', 'prompt', 'test-key'), recipe)
                self.assertEqual(list(root.iterdir()), [])
                path = root / 'valid.raw'
                self.assertEqual(request_recipe(endpoint, 'test', 'prompt', 'test-key',
                                                response_output=path), recipe)
                self.assertEqual(path.read_bytes(), valid)
                self.assertNotIn(b'test-key', path.read_bytes())
                self.assertEqual(calls[-1], 'Bearer test-key')
                before = len(calls)
                with self.assertRaises(FileExistsError):
                    request_recipe(endpoint, 'test', 'prompt', response_output=path)
                self.assertEqual(len(calls), before)
                self.assertEqual(path.read_bytes(), valid)
                bad_recipe = copy.deepcopy(recipe)
                bad_recipe['scenarios'][0]['map'] = ['#####', '#L#F#', '#####']
                for index, raw in enumerate((b'not a JSON envelope\n', envelope('not JSON'),
                                             envelope(json.dumps(bad_recipe)))):
                    response_body[0] = raw
                    path = root / ('invalid-%d.raw' % index)
                    before = len(calls)
                    with self.assertRaises(ValueError):
                        compile_recipe(request_recipe(endpoint, 'test', 'prompt', host=host,
                                                      response_output=path), host)
                    self.assertEqual(path.read_bytes(), raw)
                    self.assertEqual(len(calls), before + 1)
                response_body[0] = valid + b' ' * (1_000_000 - len(valid))
                path = root / 'maximum.raw'
                self.assertEqual(request_recipe(endpoint, 'test', 'prompt', response_output=path), recipe)
                self.assertEqual(path.read_bytes(), response_body[0])
                response_body[0] = b' ' * 1_000_001
                path = root / 'oversized.raw'
                before = len(calls)
                with self.assertRaisesRegex(ValueError, 'too large'):
                    request_recipe(endpoint, 'test', 'prompt', response_output=path)
                self.assertFalse(path.exists())
                self.assertEqual(len(calls), before + 1)
        finally:
            server.shutdown(); thread.join(); server.server_close()

    @unittest.skipUnless(os.name == 'posix', 'POSIX creation-mode contract')
    def test_raw_capture_is_owner_only_under_permissive_umask(self):
        recipe, _ = fixtures()
        valid = json.dumps({'choices': [{'message': {'content': json.dumps(recipe)}}]}).encode()
        for mask in (0o000, 0o022, 0o077):
            for raw in (valid, b'not a JSON envelope\n'):
                with self.subTest(umask=oct(mask), valid=raw == valid), tempfile.TemporaryDirectory() as tmp:
                    path = Path(tmp) / 'response.raw'
                    with patch('llm_care_scenarios.urllib.request.build_opener') as transport:
                        transport.return_value.open.return_value.__enter__.return_value.read.return_value = raw
                        previous = os.umask(mask)
                        try:
                            if raw == valid:
                                self.assertEqual(request_recipe('http://127.0.0.1:1/mock', 'test', 'prompt',
                                                                response_output=path), recipe)
                            else:
                                with self.assertRaises(ValueError):
                                    request_recipe('http://127.0.0.1:1/mock', 'test', 'prompt', response_output=path)
                        finally:
                            os.umask(previous)
                        transport.return_value.open.assert_called_once()
                    self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
                    self.assertEqual(path.read_bytes(), raw)

    @unittest.skipUnless(os.name == 'posix', 'POSIX symlink and creation-mode contract')
    def test_raw_capture_preserves_existing_files_and_symlinks_even_after_preflight(self):
        recipe, _ = fixtures()
        raw = json.dumps({'choices': [{'message': {'content': json.dumps(recipe)}}]}).encode()
        for raced in (False, True):
            for kind in ('file', 'symlink', 'dangling_symlink'):
                with self.subTest(raced=raced, kind=kind), tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    path, target = root / 'response.raw', root / 'target'
                    target.write_bytes(b'preserve target')
                    target.chmod(0o640)
                    link_target = target if kind == 'symlink' else root / 'missing'

                    def install_collision(*_):
                        if kind == 'file':
                            path.write_bytes(b'preserve existing response')
                            path.chmod(0o644)
                        else:
                            path.symlink_to(link_target)
                        return raw

                    if not raced:
                        install_collision()
                    with patch('llm_care_scenarios.urllib.request.build_opener') as transport:
                        response = transport.return_value.open.return_value.__enter__.return_value
                        response.read.side_effect = install_collision if raced else lambda *_: raw
                        with self.assertRaises(FileExistsError):
                            request_recipe('http://127.0.0.1:1/mock', 'test', 'prompt', response_output=path)
                        if raced:
                            transport.return_value.open.assert_called_once()
                            response.read.assert_called_once_with(1_000_001)
                        else:
                            transport.assert_not_called()
                    self.assertEqual(target.read_bytes(), b'preserve target')
                    self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o640)
                    if kind == 'file':
                        self.assertEqual(path.read_bytes(), b'preserve existing response')
                        self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o644)
                    else:
                        self.assertTrue(path.is_symlink())
                        self.assertEqual(os.readlink(path), str(link_target))
                    self.assertFalse((root / 'missing').exists())

    def test_raw_capture_write_failure_closes_stream_and_preserves_partial_file(self):
        raw = b'bounded response bytes'
        streams = []

        class FailingWriter:
            def __init__(self, *args, **kwargs):
                self.stream = open(*args, **kwargs)
                streams.append(self.stream)

            def __enter__(self):
                self.stream.__enter__()
                return self

            def write(self, data):
                self.stream.write(data[:5])
                raise OSError('injected write failure')

            def __exit__(self, *args):
                return self.stream.__exit__(*args)

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'response.raw'
            with patch('llm_care_scenarios.urllib.request.build_opener') as transport, \
                    patch('llm_care_scenarios.open', side_effect=FailingWriter, create=True):
                transport.return_value.open.return_value.__enter__.return_value.read.return_value = raw
                with self.assertRaisesRegex(OSError, 'injected write failure'):
                    request_recipe('http://127.0.0.1:1/mock', 'test', 'prompt', response_output=path)
                transport.return_value.open.assert_called_once()
            self.assertEqual(len(streams), 1)
            self.assertTrue(streams[0].closed)
            self.assertEqual(path.read_bytes(), raw[:5])
            if os.name == 'posix':
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)

    def test_raw_capture_opener_preserves_non_posix_mode_and_flags(self):
        # This checks flag/mode forwarding only, not actual Windows ACL privacy.
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, 'O_BINARY', 0)
        with patch('llm_care_scenarios.os.name', 'nt'), \
                patch('llm_care_scenarios.os.open', return_value=73) as create:
            self.assertEqual(_raw_response_opener('response.raw', flags), 73)
        create.assert_called_once_with('response.raw', flags, 0o666)

    def test_cli_output_collisions_fail_before_model_request(self):
        _, host = fixtures()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            host_path = root / 'host.json'
            host_path.write_text(json.dumps(host))
            bank = root / 'bank.json'
            recipe = root / 'bank.json.recipe.json'
            argv = ['llm_care_scenarios.py', '--host', str(host_path),
                    '--endpoint', 'http://127.0.0.1:1/v1/chat/completions',
                    '--model', 'test', '--output', str(bank)]
            def refused(extra=()):
                with patch('sys.argv', argv + list(extra)), patch('llm_care_scenarios.request_recipe') as request:
                    with self.assertRaises((ValueError, FileExistsError)): main()
                    request.assert_not_called()
            for existing in (bank, recipe):
                with self.subTest(existing=existing):
                    existing.write_bytes(b'preserve')
                    refused()
                    self.assertEqual(existing.read_bytes(), b'preserve')
                    existing.unlink()
            for destination in (bank, recipe, root / 'unused' / '..' / 'bank.json',
                                root / 'unused' / '..' / 'bank.json.recipe.json'):
                with self.subTest(destination=destination):
                    refused(['--response-output', str(destination)])
                    self.assertFalse(bank.exists())
                    self.assertFalse(recipe.exists())
            existing_response = root / 'response.raw'
            existing_response.write_bytes(b'preserve response')
            refused(['--response-output', str(existing_response)])
            self.assertEqual(existing_response.read_bytes(), b'preserve response')
            # A symlinked directory aliases even not-yet-created output files.
            alias = root / 'alias'
            try:
                alias.symlink_to(root, target_is_directory=True)
            except (OSError, NotImplementedError):
                return  # Windows may require elevated symlink privileges.
            for name in (bank.name, recipe.name):
                refused(['--response-output', str(alias / name)])
            for existing in (bank, recipe):
                existing.symlink_to(root / 'missing-target')
                refused()
                self.assertTrue(existing.is_symlink())
                existing.unlink()

    def test_walls_are_distinct_sampler_families_and_unreachable_rejected(self):
        recipe, host = fixtures()
        recipe['scenarios'][1]['map'] = ['#####', '#L.F#', '#...#', '#####']
        first = compile_recipe(recipe, host)
        self.assertNotEqual(first['families'][0]['layout'], first['families'][1]['layout'])
        recipe['scenarios'][1]['map'] = ['#####', '#L#F#', '#####']
        with self.assertRaises(ValueError): compile_recipe(recipe, host)
        recipe['scenarios'][1]['map'] = ['#####', '#L.F.', '#####']
        with self.assertRaises(ValueError): compile_recipe(recipe, host)

    def test_redirect_never_forwards_api_key(self):
        calls = []
        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                self.send_response(302)
                self.send_header('Location', 'http://127.0.0.1:%s/stolen' % self.server.server_port)
                self.end_headers()
            def do_GET(self):
                calls.append(self.headers.get('Authorization'))
                self.send_response(200); self.end_headers()
            def log_message(self, *_): pass
        server = HTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            with self.assertRaises(Exception):
                request_recipe('http://127.0.0.1:%s/start' % server.server_port, 'test', 'prompt', 'secret')
            self.assertEqual(calls, [])
            with self.assertRaises(ValueError):
                request_recipe('http://example.com/v1/chat/completions', 'test', 'prompt', 'secret')
        finally:
            server.shutdown(); thread.join(); server.server_close()

    def test_duplicate_json_keys_and_nonfinite_rejected(self):
        for text in ('{"a":1,"a":2}', '{"a":NaN}'):
            with self.assertRaises(ValueError): strict_json(text)

    @unittest.skipUnless(os.environ.get('ALIFE_FRONTIER_TEST_CARE') and os.environ.get('ALIFE_FRONTIER_TEST_ROUND'),
                         'explicit native binaries required')
    def test_generated_map_reaches_native_assessment(self):
        recipe, host = fixtures()
        # Use one observed family; the second remains a disjoint holdout family.
        host.update(decision_budget=2, slots=1, current_difficulty=0)
        bank = compile_recipe(recipe, host)
        discovery = [{'family': 'held', 'seed': i} for i in (301, 302, 303)]
        with tempfile.TemporaryDirectory() as tmp:
            pilot = Pilot(Path(tmp) / 'run', Path(os.environ['ALIFE_FRONTIER_TEST_CARE']),
                          Path(os.environ['ALIFE_FRONTIER_TEST_ROUND']), 2, 3, 1, bank, discovery)
            result = pilot.run()
            self.assertEqual(len(result['discovery']), 3)
            self.assertTrue(all(row['layout'] == {'rows': ['#####', '#L.F#', '#####']} for row in result['discovery']))
            self.assertIn('difficulty_after', json.loads((pilot.root / 'selection.json').read_text()))
            # Independently exercise an authored-layout learning episode even
            # when this tiny assessment finds no empirical frontier. This is a
            # disposable control, not an adaptive learning-gain claim.
            lane = pilot.root / 'layout-training-control'; lane.mkdir()
            source = pilot.root / 'initial-source'
            assessed = {row['seed'] for row in result['discovery'] + result['initial_holdouts']
                        + result.get('fixed_frozen_reference', [])}
            for arm in result['arms'].values():
                assessed.update(row['seed'] for row in arm['heldouts'])
            history = pilot.history(source, lane / 'history.json', [881001], assessed)
            training = lane / 'episode'
            receipt = pilot.command(pilot.care, ['--previous', source, '--output', training,
                '--seed-history', history, '--seed', 401, '--lesson', 'feeding',
                '--food-need-condition', 'newborn_v1', '--decisions', 2,
                '--layout-json', json.dumps(bank['families'][0]['layout'], separators=(',', ':'))], lane)
            self.assertEqual(receipt['episode']['layout'], bank['families'][0]['layout'])
            verified = pilot.command(pilot.care, ['--validate-output', training], lane)
            self.assertEqual(verified['episode']['layout'], bank['families'][0]['layout'])

    def test_seed_overlap_and_unsupported_condition_reject(self):
        recipe, host = fixtures(); host['holdout_seeds']['held'] = [101]
        with self.assertRaises(ValueError): compile_recipe(recipe, host)
        recipe, host = fixtures(); recipe['scenarios'][0]['condition'] = 'invalid_condition'
        with self.assertRaises(ValueError): compile_recipe(recipe, host)

if __name__ == '__main__': unittest.main()
