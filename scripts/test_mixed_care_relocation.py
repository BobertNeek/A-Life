"""Saved relocation admission contracts; no native/GPU execution is mocked as a pass."""
import copy
import struct
import unittest

import run_mixed_care_serial as mixed


def fixture(*, meal_row=3, relocate=True):
    receipt = dict(seed=100, lesson="relocated_food", founder_seed_base=42,
        biological_objective_version=2, objective_state_reset=False,
        policy_version=11, old_asset_digest="old", new_asset_digest="new",
        terminal_death_tick=None, speech_target_rows=0, requested_training_ticks=4,
        trained_replay_rows=4, training_ticks=4, captured_replay_rows=5,
        bootstrap_replay_rows=1, completed_epochs=1, actor_optimizer_updates=2,
        value_optimizer_updates=2, prepared_held_food_opportunities=0,
        prepared_reachable_food_opportunities=0, held_food_consumption_events=0,
        trained_held_food_consumption_events=0, consumed_events=int(meal_row is not None))
    initial, moved = dict(x=1.0, y=0.0, z=0.0), dict(x=0.0, y=0.0, z=1.0)
    loose, held, eaten = dict(owner=None, consumed=False), dict(owner=7, consumed=False), dict(owner=7, consumed=True)
    rows = []
    for i in range(5):
        acquisition, meal = i == 2 and meal_row is not None, i == meal_row
        before = held if meal else loose
        after = eaten if meal else held if acquisition else loose
        rows.append(dict(row=i, organism_id=7, lesson_target=9, behavior="OnPolicy",
            loss_row=i < 4, food_relocated=relocate and i >= 2, food_visible=True,
            decision_tick=i, outcome_tick=i+1, sequence_id=i+1, dispatch_generation=i,
            frame_digest=[1, 2, 3, 4], food_position_before_decision=moved if relocate and i >= 2 else initial,
            food_position_after_outcome=moved if relocate and i >= 2 else initial,
            lesson_target_ownership_before_decision=before,
            lesson_target_ownership_after_outcome=after,
            selected_action=dict(organism_id=7, action_id=210 if meal or acquisition else 0, target_entity=9),
            selected_commands=None, channel_outcomes=None,
            physical=dict(contact="Consumed" if meal else "Touch" if acquisition else "None", target_entity=9),
            ordinary_acquisition=acquisition, nourished_meal=meal,
            physiology=dict(before_energy=0.2, after_energy=0.5, before_hunger=0.6, after_hunger=0.2,
                before_health=1.0, after_health=1.0, before_injury=0.0, after_injury=0.0, before_pain=0.0, after_pain=0.0)))
    post_meals = int(relocate and meal_row is not None and 2 < meal_row < 4)
    receipt["food_relocation"] = dict(schema=1, organism=7, target=9, setup_tick=0,
        initial_position=initial, relocation_target=moved,
        relocation=dict(row=2, decision_tick=2, from_position=initial, to_position=moved,
            ownership_before=loose, ownership_after=loose) if relocate else None,
        captured_rows=5, trained_rows=4, acquisition_rows=[2] if meal_row is not None else [],
        nourished_meal_rows=[meal_row] if meal_row is not None else [],
        trained_post_relocation_acquisitions=int(relocate and meal_row is not None),
        trained_post_relocation_nourished_meals=post_meals,
        trained_pre_relocation_visible_rows=2 if relocate else 4,
        trained_post_relocation_visible_rows=2 if relocate else 0,
        food_outcome=dict(organism=7, target=9, ordinary_acquisition_events=int(meal_row is not None),
            explicit_grab_acquisition_events=0, consumption_events=int(meal_row is not None),
            nourished_meal_events=int(meal_row is not None)),
        lesson_completed=bool(post_meals), rows=rows)
    return receipt


class RelocatedCareTests(unittest.TestCase):
    def validate(self, receipt):
        mixed.validate_receipt(receipt, dict(founder_seed_base=42, policy_version=10, new_asset_digest="old"),
                               dict(lesson="relocated_food", decisions=4), 100)

    def test_exact_own_acquisition_and_trained_meal_are_admitted(self):
        self.validate(fixture())

    def f32_fixture(self):
        receipt = fixture()
        evidence = receipt["food_relocation"]
        initial = dict(x=6.294225, y=0.0, z=-0.30863175)
        moved = dict(x=2.6913683, y=0.0, z=-3.2942252)
        evidence.update(initial_position=initial, relocation_target=moved)
        evidence["relocation"].update(from_position=initial, to_position=moved)
        for row in evidence["rows"]:
            position = moved if row["row"] >= 2 else initial
            widened = {axis: struct.unpack("<f", struct.pack("<f", number))[0]
                       for axis, number in position.items()}
            row.update(food_position_before_decision=widened.copy(), food_position_after_outcome=widened.copy())
        return receipt

    def test_native_f32_serde_and_widened_trace_positions_bind_exactly(self):
        receipt = self.f32_fixture()
        evidence = receipt["food_relocation"]
        self.assertNotEqual(evidence["relocation"]["to_position"], evidence["rows"][2]["food_position_before_decision"])
        self.validate(receipt)

    def test_adjacent_native_f32_position_still_contradicts_relocation(self):
        for row, field in ((2, "food_position_before_decision"), (1, "food_position_after_outcome")):
            with self.subTest(row=row):
                receipt = self.f32_fixture()
                position = receipt["food_relocation"]["rows"][row][field]
                bits = struct.unpack("<I", struct.pack("<f", position["x"]))[0]
                position["x"] = struct.unpack("<f", struct.pack("<I", bits + 1))[0]
                with self.assertRaises(ValueError):
                    self.validate(receipt)

    def test_non_native_boundary_position_is_rejected(self):
        for number in (float("inf"), float("nan"), 1e300, True):
            with self.subTest(number=number):
                receipt = self.f32_fixture()
                receipt["food_relocation"]["rows"][2]["food_position_before_decision"]["x"] = number
                with self.assertRaises(ValueError):
                    self.validate(receipt)

    def test_bundle_manipulation_is_used_instead_of_representative_action(self):
        receipt = fixture()
        for row in receipt["food_relocation"]["rows"]:
            # Future native diagnostics include all channels and complete commands.
            # This host fixture tests compatibility/accounting, not world execution.
            row["selected_commands"] = [dict(channel="Manipulation", primitive=row["selected_action"]["action_id"], target=dict(entity=9)),
                dict(channel="Locomotion", primitive=105, target=None),
                dict(channel="Orientation", primitive=604, target=None)]
            for command in row["selected_commands"]:
                command.update(direction=dict(x=0., y=0., z=1.), intensity=.5, duration_ticks=1,
                               stand_off_distance=0., payload=dict(values=[]), confidence=1., coordination_group=0)
            row["channel_outcomes"] = [dict(channel="Manipulation", physical=row["physical"]),
                dict(channel="Locomotion", physical=dict(contact="Moved", target_entity=None))]
            row["applied_motor_channels"] = [dict(command=row["selected_commands"][1], phase="Completed",
                physical=row["channel_outcomes"][1]["physical"], observation=None)]
            row["body_before_decision"] = dict(entity=8, position=dict(x=0., y=0., z=0.),
                velocity=dict(x=0., y=0., z=0.), body_yaw=.2, head_yaw=.3, gaze_yaw=.5)
            row["body_after_outcome"] = None
            row["selected_action"]["action_id"] = 0
            row["selected_action"]["target_entity"] = None
            row["physical"] = dict(contact="None", target_entity=None)
        before = copy.deepcopy(receipt)
        self.validate(receipt)
        self.assertEqual(receipt, before)

    def test_valid_miss_and_no_relocation_are_learning_not_false_success(self):
        for relocate in (True, False):
            with self.subTest(relocate=relocate):
                receipt = fixture(meal_row=None, relocate=relocate)
                self.validate(receipt)
                self.assertFalse(receipt["food_relocation"]["lesson_completed"])

    def test_bootstrap_meal_is_reported_but_not_completion(self):
        receipt = fixture(meal_row=4)
        self.validate(receipt)
        self.assertFalse(receipt["food_relocation"]["lesson_completed"])
        receipt["food_relocation"]["lesson_completed"] = True
        with self.assertRaises(ValueError):
            self.validate(receipt)

    def test_rejects_fabricated_binding_or_unearned_completion(self):
        changes = {
            "missing": lambda r: r.pop("food_relocation"),
            "target": lambda r: r["food_relocation"]["food_outcome"].update(target=99),
            "foreign learner": lambda r: r["food_relocation"]["rows"][2].update(organism_id=99),
            "loss row": lambda r: r["food_relocation"]["rows"][4].update(loss_row=True),
            "no physical move": lambda r: r["food_relocation"]["relocation"].update(to_position=dict(x=1., y=0., z=0.)),
            "player held": lambda r: r["food_relocation"]["relocation"]["ownership_before"].update(owner=99),
            "early move": lambda r: r["food_relocation"]["relocation"].update(decision_tick=1),
            "contradictory post position": lambda r: r["food_relocation"]["rows"][2].update(food_position_before_decision=dict(x=1., y=0., z=0.)),
            "contradictory pre position": lambda r: r["food_relocation"]["rows"][1].update(food_position_after_outcome=dict(x=0., y=0., z=1.)),
            "assessment as training": lambda r: r["food_relocation"].update(assessment=dict(rows=4, completed=True)),
            "wrong pickup target": lambda r: r["food_relocation"]["rows"][2]["physical"].update(target_entity=99),
            "player pickup": lambda r: r["food_relocation"]["rows"][2]["lesson_target_ownership_after_outcome"].update(owner=99),
            "wrong meal target": lambda r: r["food_relocation"]["rows"][3]["selected_action"].update(target_entity=99),
            "no physiology": lambda r: r["food_relocation"]["rows"][3].update(physiology=None),
            "no benefit": lambda r: r["food_relocation"]["rows"][3]["physiology"].update(after_energy=.1, after_hunger=.8),
            "fresh harm": lambda r: r["food_relocation"]["rows"][3]["physiology"].update(after_health=.5),
            "unseen post": lambda r: r["food_relocation"]["rows"][3].update(food_visible=False),
            "off policy": lambda r: r["food_relocation"]["rows"][3].update(behavior="Teacher"),
            "duplicate meal": lambda r: r["food_relocation"].update(nourished_meal_rows=[3, 3]),
            "fictional acquisition": lambda r: r["food_relocation"].update(trained_post_relocation_acquisitions=2),
        }
        for name, change in changes.items():
            with self.subTest(name=name):
                receipt = copy.deepcopy(fixture())
                change(receipt)
                with self.assertRaises(ValueError):
                    self.validate(receipt)

    def test_summary_separates_exact_meals_bootstrap_and_relocation_completion(self):
        receipts = [fixture(), fixture(meal_row=4), fixture(meal_row=None)]
        run = dict(plan=dict(source="original"), attempts=[dict(status="accepted", receipt=r, directory=str(i))
                                                          for i, r in enumerate(receipts)])
        result = mixed.summary(run)
        total = result["per_lesson"]["relocated_food"]
        self.assertEqual(total["episodes"], 3)
        self.assertEqual(total["exact_nourishing_consumption_events"], 2)
        self.assertEqual(total["trained_exact_nourishing_consumption_events"], 1)
        self.assertEqual(total["trained_post_relocation_nourished_meals"], 1)
        self.assertEqual(total["completed_relocation_episodes"], 1)
        self.assertFalse(result["promoted"])

    def test_old_feeding_receipt_remains_valid_without_new_field(self):
        receipt = fixture()
        receipt.update(lesson="feeding")
        receipt.pop("food_relocation")
        mixed.validate_receipt(receipt, dict(founder_seed_base=42, policy_version=10, new_asset_digest="old"),
                               dict(lesson="feeding", decisions=4), 100)

    def test_schedule_admits_real_relocation_without_new_mastery_threshold(self):
        rows = mixed.schedule_rows(dict(schema=1, steps=[dict(lesson="relocated_food", episodes=2, decisions=64)]))
        self.assertEqual(rows, [dict(lesson="relocated_food", decisions=64)] * 2)


if __name__ == "__main__":
    unittest.main()
