"""Pure scalar/algebra tests; no native brain, worlds, or state files accessed."""
import unittest
from circuit_proposal import *

class StructuralAndAlgebraTests(unittest.TestCase):
    def test_unique_append_only_addresses(self):
        addresses = [n for p in ALL_PAIRS for n in (p.signal, p.reference)]
        self.assertEqual(len(addresses), 52)
        self.assertEqual(len(set(addresses)), 52)
        self.assertEqual(set(addresses), set(range(2048, 2100)))

    def test_readout_zero_sum_pairs(self):
        rs = readouts()
        for a, b in zip(rs[::2], rs[1::2]):
            self.assertEqual((a.family, a.lane), (b.family, b.lane))
            self.assertEqual(a.weight, -b.weight)
            self.assertEqual(a.source + 1, b.source)

    def test_shared_offset_cannot_reverse_nonnegative_branch_advantage(self):
        for background in [-4., -1., -.275, 0., .275, 1., 4.]:
            for gain in [0., .25, .5, 1., 2.]:
                for leak in [0., .1, .5, 1.]:
                    a, b = scalar_matched_step(-.3, -.3, .2, 0., background, gain, leak)
                    self.assertGreaterEqual(a, b)
        # This proves one-step order only. Matched initial state and background
        # give induction when branch advantage stays nonnegative. It does NOT
        # prove useful gain, recurrent stability, or world behavior.

    def test_no_locomotion_added_readout_exactly_neutral(self):
        no_loco = [0.]*54
        no_loco[23] = 1.  # Current intrinsic basis.
        state = {p.signal: 1.0 for p in ALL_PAIRS}
        state.update({p.reference: -.7 for p in ALL_PAIRS})
        self.assertEqual(candidate_delta(state, 'Approach', no_loco), 0.)

    def test_recovery_only_lowers_observed_movement(self):
        object_features = [0.]*54
        object_features[36] = 1.
        state = {REST.signal: .2, REST.reference: -.1}
        self.assertLess(candidate_delta(state, 'Approach', object_features), 0.)
        self.assertLess(candidate_delta(state, 'Avoid', object_features), 0.)
        self.assertEqual(candidate_delta(state, 'Ingest', object_features), 0.)

    def test_atp_and_extension_sign(self):
        b = next(x for x in branches() if x.target == ATP_LOW.signal)
        self.assertEqual(b.inputs, ((AFFERENTS[6].signal, -4.0), (AFFERENTS[6].reference, 4.0)))
        used = {r.source for r in readouts()}
        self.assertFalse(used & {HIGH[10].signal, HIGH[10].reference})
        downstream = [b for b in branches() if b.target >= REST.signal]
        self.assertFalse(any(n in [HIGH[10].signal, HIGH[10].reference]
                             for b in downstream for n, w in b.inputs))


# Refinement tests are written but were NOT run in the no-execution refinement.
class MediationConstructionTests(unittest.TestCase):
    def test_m0_has_only_execution_sources_in_new_head(self):
        m = mediation_manifest(False)
        self.assertEqual(len(m['execution_pairs']), 20)
        self.assertEqual(len(m['new_arbitration_readout_head']), 50)
        self.assertTrue(all(2100 <= r['source'] < 2140
                            for r in m['new_arbitration_readout_head']))
        self.assertFalse(m['direct_controller_readouts_active'])
        self.assertEqual(m['extra_cost_before_tile_padding']['branches'], 70)
        self.assertEqual(sum(len(b['inputs']) for b in m['new_branches']), 184)

    def test_m1_keeps_each_original_proposal_individually(self):
        m = mediation_manifest(True)
        ps = m['legacy_execution_pairs']
        self.assertEqual([p['proposal_source'] for p in ps], list(range(1728,1856)))
        self.assertEqual([p['signal'] for p in ps], list(range(2144,2400,2)))
        self.assertEqual(len(m['legacy_copy_recurrent_rows']), 256)
        self.assertEqual(m['allocated_synapse_rows_if_class5_base34048'], 43058)

    def test_m1_constructor_requires_the_exact_complete_bank(self):
        rows = [dict(family=f, local_motor=m, lane=l, weight=.25,
                     persistent_address=(f,m,l))
                for f in FAMILIES for m in range(16)
                for l in list(range(24))+list(range(36,46))]
        out = materialize_legacy_readouts(rows)
        self.assertEqual(len(out), 8704)
        self.assertTrue(all(a.weight == -b.weight for a,b in zip(out[::2],out[1::2])))
        with self.assertRaises(ValueError):
            materialize_legacy_readouts(rows[:-1])
        with self.assertRaises(ValueError):
            materialize_legacy_readouts(rows+[rows[0]])

if __name__ == '__main__':
    unittest.main()
