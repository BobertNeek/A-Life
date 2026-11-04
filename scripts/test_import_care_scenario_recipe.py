import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from import_care_scenario_recipe import import_recipe, MAX_INPUT_BYTES
from llm_care_scenarios import LEGEND, RANDOM_ROLL, compile_recipe

SCRIPT = Path(__file__).with_name('import_care_scenario_recipe.py')


class ImportRecipeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.recipe = {'schema': 1, 'legend': LEGEND, 'scenarios': [
            {'id': 'feeding', 'map': ['#####', '#L.F#', '#####'],
             'condition': 'newborn_v1', 'difficulty': 0,
             'goal': 'nourish_and_survive', 'random_roll': RANDOM_ROLL}]}
        self.host = {'decision_budget': 64, 'slots': 1, 'sampler_seed': 42,
                     'current_difficulty': 0, 'pool_seeds': {'feeding': [101]},
                     'holdout_seeds': {'feeding': [201]}}
        self.r, self.h, self.out = (self.root / n for n in ('recipe.json', 'host.json', 'bank.json'))
        self.r.write_text(json.dumps(self.recipe))
        self.h.write_text(json.dumps(self.host))

    def cli(self, recipe=None, host=None, output=None):
        return subprocess.run([sys.executable, str(SCRIPT), '--recipe', str(recipe or self.r),
                               '--host', str(host or self.h), '--output', str(output or self.out)],
                              capture_output=True, text=True, timeout=10)

    def test_cli_bank_and_receipt_hash_exact_bytes(self):
        result = self.cli()
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt['source'], 'external_json')
        self.assertEqual(receipt['families'], 1)
        for key, path in [('recipe', self.r), ('host', self.h), ('bank', self.out)]:
            self.assertEqual(receipt[key + '_sha256'], hashlib.sha256(path.read_bytes()).hexdigest())
        self.assertEqual(json.loads(self.out.read_bytes()), compile_recipe(self.recipe, self.host))
        self.assertNotIn('codex', result.stdout.lower())
        self.assertEqual(set(self.root.iterdir()), {self.r, self.h, self.out})
        original = self.out.read_bytes()
        self.assertNotEqual(self.cli().returncode, 0)
        self.assertEqual(self.out.read_bytes(), original)

    def test_bad_inputs_create_no_output(self):
        bad = [b'not JSON', b'{"schema":1,"schema":1}', b'{"x":NaN}',
               b'{"x":Infinity}', b'{"x":1e999}', b'\xff',
               b' ' * (MAX_INPUT_BYTES + 1)]
        for which in (self.r, self.h):
            original = which.read_bytes()
            for raw in bad:
                with self.subTest(which=which.name, raw=raw[:40]):
                    which.write_bytes(raw)
                    result = self.cli()
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, '')
                    self.assertFalse(self.out.exists())
            which.write_bytes(original)

    def test_semantic_rejection_before_writing(self):
        for mutate in (lambda r: r.update(schema=True),
                       lambda r: r['scenarios'][0].update(map=['#####', '#L#F#', '#####']),
                       lambda r: r['scenarios'][0].update(reward=100),
                       lambda r: r['scenarios'][0].update(id=[])):
            recipe = copy.deepcopy(self.recipe); mutate(recipe)
            self.r.write_text(json.dumps(recipe))
            self.assertNotEqual(self.cli().returncode, 0)
            self.assertFalse(self.out.exists())
        self.r.write_text(json.dumps(self.recipe))
        self.host['holdout_seeds']['feeding'] = [101]
        self.h.write_text(json.dumps(self.host))
        self.assertNotEqual(self.cli().returncode, 0)
        self.assertFalse(self.out.exists())

    def test_collisions_preserve_inputs_and_existing_output(self):
        for path in (self.r, self.h):
            original = path.read_bytes()
            self.assertNotEqual(self.cli(output=path).returncode, 0)
            self.assertEqual(path.read_bytes(), original)
        self.assertNotEqual(self.cli(host=self.r).returncode, 0)
        alias = self.root / 'recipe-hardlink.json'
        os.link(self.r, alias)
        self.assertNotEqual(self.cli(host=alias).returncode, 0)
        self.out.write_bytes(b'prior result')
        self.assertNotEqual(self.cli().returncode, 0)
        self.assertEqual(self.out.read_bytes(), b'prior result')

    def test_symlinks_and_parent_aliases_reject(self):
        alias = self.root / 'alias'
        try:
            alias.symlink_to(self.root, target_is_directory=True)
        except (OSError, NotImplementedError):
            self.skipTest('symlinks unavailable')
        for kwargs in ({'recipe': alias / self.r.name}, {'host': alias / self.h.name},
                       {'output': alias / self.r.name}, {'output': alias / 'new.json'}):
            self.assertNotEqual(self.cli(**kwargs).returncode, 0)
        self.out.symlink_to(self.root / 'missing')
        self.assertNotEqual(self.cli().returncode, 0)
        self.assertTrue(self.out.is_symlink())
        self.assertFalse((self.root / 'missing').exists())

    def test_directory_and_fifo_inputs_reject(self):
        self.assertNotEqual(self.cli(recipe=self.root).returncode, 0)
        if hasattr(os, 'mkfifo'):
            fifo = self.root / 'fifo'
            os.mkfifo(fifo)
            self.assertNotEqual(self.cli(recipe=fifo).returncode, 0)
        self.assertFalse(self.out.exists())

    def test_collision_race_does_not_overwrite(self):
        real_compile = compile_recipe
        def collision(*args):
            result = real_compile(*args)
            self.out.write_bytes(b'concurrent result')
            return result
        with patch('import_care_scenario_recipe.compile_recipe', side_effect=collision):
            with self.assertRaises(FileExistsError):
                import_recipe(self.r, self.h, self.out)
        self.assertEqual(self.out.read_bytes(), b'concurrent result')

    def test_exact_limit_allowed(self):
        raw = self.r.read_bytes()
        self.r.write_bytes(raw + b' ' * (MAX_INPUT_BYTES - len(raw)))
        result = self.cli()
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
