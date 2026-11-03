import copy
import os
import tempfile
from pathlib import Path
from run_care_frontier_fixture import Pilot
import unittest
import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from llm_care_scenarios import request_recipe, strict_json
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
                                   'fixture-model', 'recipe instructions')
            self.assertEqual(compile_recipe(value, host)['families'][0]['layout'], {'rows': ['#####', '#L.F#', '#####']})
            self.assertEqual(calls[0]['model'], 'fixture-model')
        finally:
            server.shutdown(); thread.join(); server.server_close()

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
