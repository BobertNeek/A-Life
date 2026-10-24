"""Compact Feeding receipt contracts; these synthetic counts are not native biology evidence."""
import copy
import unittest

import run_mixed_care_serial as mixed


def fixture(*, meal_row=2, measured=True, generic=1):
    receipt = dict(seed=100, lesson="feeding", founder_seed_base=42,
        biological_objective_version=2, objective_state_reset=False,
        policy_version=11, old_asset_digest="old", new_asset_digest="new",
        terminal_death_tick=None, speech_target_rows=0, requested_training_ticks=4,
        trained_replay_rows=4, training_ticks=4, captured_replay_rows=5,
        bootstrap_replay_rows=1, completed_epochs=1, actor_optimizer_updates=2,
        value_optimizer_updates=2, prepared_held_food_opportunities=0,
        prepared_reachable_food_opportunities=0, held_food_consumption_events=0,
        trained_held_food_consumption_events=0, consumed_events=generic)
    if measured:
        receipt["feeding_food"] = dict(captured_rows=5, trained_rows=4,
            nourished_meal_rows=[] if meal_row is None else [meal_row],
            food_outcome=dict(organism=7, target=9, explicit_grab_acquisition_events=0,
                ordinary_acquisition_events=int(meal_row is not None),
                consumption_events=int(meal_row is not None),
                nourished_meal_events=int(meal_row is not None)))
    return receipt


def summarize(receipts):
    run = dict(plan=dict(source="original"), attempts=[dict(status="accepted", receipt=receipt, directory=str(index))
                                                     for index, receipt in enumerate(receipts)])
    return mixed.summary(run)["per_lesson"]["feeding"]


class FeedingCareTests(unittest.TestCase):
    def validate(self, receipt):
        mixed.validate_receipt(receipt, dict(founder_seed_base=42, policy_version=10, new_asset_digest="old"),
                               dict(lesson="feeding", decisions=4), 100)

    def test_exact_trained_meal_is_separate_from_generic_consumption(self):
        receipt = fixture(generic=3)
        self.validate(receipt)
        total = summarize([receipt])
        self.assertEqual(total["generic_consumed_events"], 3)
        self.assertEqual(total["exact_nourishing_consumption_events"], 1)
        self.assertEqual(total["trained_exact_nourishing_consumption_events"], 1)
        self.assertEqual(total["exact_nourishment_measured_episodes"], 1)
        self.assertEqual(total["exact_nourishment_unknown_episodes"], 0)

    def test_bootstrap_meal_is_captured_but_not_trained(self):
        receipt = fixture(meal_row=4)
        self.validate(receipt)
        total = summarize([receipt])
        self.assertEqual(total["exact_nourishing_consumption_events"], 1)
        self.assertEqual(total["trained_exact_nourishing_consumption_events"], 0)

    def test_same_target_regrowth_can_have_distinct_trained_and_bootstrap_meals(self):
        # Synthetic compact evidence represents two independently observed meals.
        # Native ecology tests prove real regrowth and zero-nutrition reacquisition.
        receipt = fixture(generic=2)
        receipt["feeding_food"].update(nourished_meal_rows=[1, 4])
        receipt["feeding_food"]["food_outcome"].update(
            ordinary_acquisition_events=2, consumption_events=2, nourished_meal_events=2)
        self.validate(receipt)
        total = summarize([receipt])
        self.assertEqual(total["generic_consumed_events"], 2)
        self.assertEqual(total["exact_nourishing_consumption_events"], 2)
        self.assertEqual(total["trained_exact_nourishing_consumption_events"], 1)

    def test_measured_zero_stays_zero_despite_generic_consumption(self):
        receipt = fixture(meal_row=None, generic=3)
        self.validate(receipt)
        total = summarize([receipt])
        self.assertEqual(total["exact_nourishing_consumption_events"], 0)
        self.assertEqual(total["trained_exact_nourishing_consumption_events"], 0)
        self.assertEqual(total["generic_consumed_events"], 3)

    def test_missing_or_null_old_evidence_remains_unknown_and_unchanged(self):
        for explicit_null in (False, True):
            with self.subTest(explicit_null=explicit_null):
                receipt = fixture(measured=False, generic=3)
                if explicit_null:
                    receipt["feeding_food"] = None
                before = copy.deepcopy(receipt)
                self.validate(receipt)
                total = summarize([receipt])
                self.assertIsNone(total["exact_nourishing_consumption_events"])
                self.assertIsNone(total["trained_exact_nourishing_consumption_events"])
                self.assertEqual(total["known_exact_nourishing_consumption_events"], 0)
                self.assertEqual(total["exact_nourishment_unknown_episodes"], 1)
                self.assertEqual(total["generic_consumed_events"], 3)
                self.assertEqual(receipt, before)

    def test_mixed_old_new_totals_are_unknown_with_explicit_known_subtotal(self):
        total = summarize([fixture(), fixture(meal_row=4), fixture(measured=False, generic=2)])
        self.assertIsNone(total["exact_nourishing_consumption_events"])
        self.assertIsNone(total["trained_exact_nourishing_consumption_events"])
        self.assertEqual(total["known_exact_nourishing_consumption_events"], 2)
        self.assertEqual(total["known_trained_exact_nourishing_consumption_events"], 1)
        self.assertEqual(total["exact_nourishment_measured_episodes"], 2)
        self.assertEqual(total["exact_nourishment_unknown_episodes"], 1)
        self.assertEqual(total["episodes"], 3)
        self.assertEqual(total["generic_consumed_events"], 4)

    def test_no_accepted_feeding_has_an_empty_known_total(self):
        total = summarize([])
        self.assertEqual(total["exact_nourishing_consumption_events"], 0)
        self.assertEqual(total["exact_nourishment_unknown_episodes"], 0)

    def test_acquisition_or_regrab_counts_never_create_a_meal(self):
        receipt = fixture(meal_row=None, generic=0)
        receipt["feeding_food"]["food_outcome"].update(ordinary_acquisition_events=3, explicit_grab_acquisition_events=2)
        self.validate(receipt)
        self.assertEqual(summarize([receipt])["exact_nourishing_consumption_events"], 0)

    def test_invalid_optional_evidence_is_rejected(self):
        changes = {
            "nonobject evidence": lambda r: r.update(feeding_food=False),
            "nonobject outcome": lambda r: r["feeding_food"].update(food_outcome=[]),
            "missing row evidence": lambda r: r["feeding_food"].pop("nourished_meal_rows"),
            "extra unsupported evidence": lambda r: r["feeding_food"].update(assumed_meals=1),
            "captured binding": lambda r: r["feeding_food"].update(captured_rows=6),
            "trained binding": lambda r: r["feeding_food"].update(trained_rows=3),
            "boolean trained": lambda r: r["feeding_food"].update(trained_rows=True),
            "missing organism": lambda r: r["feeding_food"]["food_outcome"].pop("organism"),
            "invalid organism": lambda r: r["feeding_food"]["food_outcome"].update(organism=0),
            "invalid target": lambda r: r["feeding_food"]["food_outcome"].update(target=True),
            "negative acquisition": lambda r: r["feeding_food"]["food_outcome"].update(ordinary_acquisition_events=-1),
            "excess acquisition": lambda r: r["feeding_food"]["food_outcome"].update(ordinary_acquisition_events=6),
            "excess explicit grab": lambda r: r["feeding_food"]["food_outcome"].update(explicit_grab_acquisition_events=2),
            "consumed exceeds generic": lambda r: r["feeding_food"]["food_outcome"].update(consumption_events=2),
            "meals exceed consumption": lambda r: r["feeding_food"]["food_outcome"].update(nourished_meal_events=2),
            "boolean meal": lambda r: r["feeding_food"]["food_outcome"].update(nourished_meal_events=True),
            "missing meal row": lambda r: r["feeding_food"].update(nourished_meal_rows=[]),
            "row out of range": lambda r: r["feeding_food"].update(nourished_meal_rows=[5]),
            "negative row": lambda r: r["feeding_food"].update(nourished_meal_rows=[-1]),
            "boolean row": lambda r: r["feeding_food"].update(nourished_meal_rows=[True]),
            "nonlist rows": lambda r: r["feeding_food"].update(nourished_meal_rows="2"),
        }
        for name, change in changes.items():
            with self.subTest(name=name):
                receipt = fixture()
                change(receipt)
                with self.assertRaises(ValueError):
                    self.validate(receipt)

    def test_duplicate_or_unsorted_rows_cannot_double_credit_a_meal(self):
        for rows in ([2, 2], [3, 2]):
            with self.subTest(rows=rows):
                receipt = fixture(generic=2)
                receipt["feeding_food"].update(nourished_meal_rows=rows)
                receipt["feeding_food"]["food_outcome"].update(consumption_events=2, nourished_meal_events=2)
                with self.assertRaises(ValueError):
                    self.validate(receipt)

    def test_feeding_evidence_cannot_be_attached_to_another_lesson(self):
        receipt = fixture()
        receipt["lesson"] = "relocated_food"
        with self.assertRaises(ValueError):
            mixed.validate_feeding_food(receipt)


if __name__ == "__main__":
    unittest.main()
