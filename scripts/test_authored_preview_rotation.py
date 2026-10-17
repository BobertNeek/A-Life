"""Small stdlib guards for the renderer block and its pinned source transition."""
import copy
import unittest

import check_authored_preview_rotation as rotation
import check_fern_coastal_composition as composition


class AuthoredPreviewRotationTests(unittest.TestCase):
    def test_each_renderer_activates_xyz_before_authored_yaw(self):
        for renderer in rotation.RENDERERS:
            with self.subTest(renderer=renderer):
                block = rotation.placement_statements((rotation.ROOT / renderer).read_text())
                self.assertEqual([rotation.target_attribute(row) for row in block],
                                 ['location', 'rotation_mode', 'rotation_euler', 'scale'])
                self.assertEqual(block[1].value.value, 'XYZ')

    def test_historical_euler_only_blocks_reproduce_the_regression_fixture(self):
        for renderer in rotation.RENDERERS:
            source = (rotation.ROOT / renderer).read_text().replace("o.rotation_mode='XYZ';", '')
            with self.subTest(renderer=renderer):
                block = rotation.placement_statements(source)
                self.assertEqual([rotation.target_attribute(row) for row in block],
                                 ['location', 'rotation_euler', 'scale'])
                rotation.placement_code(source)

    def test_rejects_an_unreviewed_renderer_transition(self):
        profile, fp, cp, before, consolidated = composition.load_profiles(composition.ROOT)
        profile = copy.deepcopy(profile)
        profile['renderer_corrections'][rotation.RENDERERS[0]]['corrected_sha256'] = '0' * 64
        with self.assertRaisesRegex(AssertionError, 'Unreviewed renderer correction'):
            composition.validate_sources(profile, fp, cp, before, consolidated)

    def test_rejects_a_stale_executable_hash(self):
        profile, fp, cp, before, consolidated = composition.load_profiles(composition.ROOT)
        profile = copy.deepcopy(profile)
        profile['sources'][rotation.RENDERERS[0]] = profile['renderer_corrections'][rotation.RENDERERS[0]]['historical_sha256']
        with self.assertRaisesRegex(AssertionError, 'Corrected renderer pin changed'):
            composition.validate_sources(profile, fp, cp, before, consolidated)


if __name__ == '__main__':
    unittest.main()
