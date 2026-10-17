"""Guard the exact CI-only successor without rewriting historical profiles."""
import copy
import unittest

import check_fern_coastal_composition as check


class MainIntegrationTests(unittest.TestCase):
    def test_fixed_historical_profile_and_only_ci_pin_superseded(self):
        historical, integration = check.load_main_integration(check.ROOT)
        current = check.apply_main_integration(historical, integration)
        expected = copy.deepcopy(historical)
        expected['sources']['.github/workflows/ci.yml'] = integration['workflow_transition']['current_sha256']
        self.assertEqual(current, expected)
        self.assertEqual(historical['sources']['.github/workflows/ci.yml'],
                         integration['workflow_transition']['historical_sha256'])

    def test_rejects_an_unreviewed_historical_ci_pin(self):
        historical, integration = check.load_main_integration(check.ROOT)
        historical['sources']['.github/workflows/ci.yml'] = '0' * 64
        with self.assertRaisesRegex(AssertionError, 'Historical CI source pin'):
            check.apply_main_integration(historical, integration)

    def test_current_ci_retains_all_unrelated_main_steps(self):
        text = (check.ROOT / '.github/workflows/ci.yml').read_text()
        for name in ('Run selection highlight parity and neighboring pointer CPU tests',
                     'Run live food projection and carried-object CPU tests',
                     'Run selected-creature HUD CPU data and layout tests',
                     'Serial-cycle stage timing summary CPU tests',
                     'Grab seed reservation CPU tests',
                     'Adaptive game exploration shader and host CPU contracts',
                     'Familiar-object unresolved-gap input chain CPU regression'):
            with self.subTest(step=name):
                self.assertIn(name, text)


if __name__ == '__main__':
    unittest.main()
