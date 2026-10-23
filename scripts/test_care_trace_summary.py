"""Synthetic evidence only: no native execution or private checkpoint fixtures."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import summarize_care_trace as care


def slot(index, tracked):
    return dict(slot_index=index, tracked_object_id=tracked, bearing=[0, 1],
                relative_velocity=[0, 0, 0], chemical=[0, 0, 0], proprioception=[0, 0],
                color=[.5, .5, .5], material=[.5, .5, .5], shape=[.5, .5, .5],
                terrain=[0, 0], distance=.1, contact=0, confidence=1, temperature=0)


def command(channel, primitive, target):
    return dict(channel=channel, primitive=primitive,
                target=None if target is None else dict(entity=target, position=None))


def row(tick, *, visible=True, selected=None, before=None, after=None, contact="None", physical_target=2):
    own = dict(owner=None, consumed=False)
    before = own if before is None else before
    after = before if after is None else after
    return dict(tick=tick, decision_tick=tick, outcome_tick=tick + 1,
                organism_id=1, lesson_target=2,
                object_slots=[slot(0, 9)] if visible else [],
                candidates=[dict(index=0, action=101 if visible else 1,
                                 family="Approach" if visible else "Idle", logit=0,
                                 observation={"ObjectSlot": 0} if visible else "None",
                                 target=2 if visible else None,
                                 target_position=dict(x=.6, y=0, z=0) if visible else None)],
                position=dict(x=0, y=0, z=0), chosen_action=601, chosen_target=None,
                selected_commands=[] if selected is None else selected,
                channel_outcomes=[dict(channel="Manipulation", physical=dict(
                    contact=contact, target_entity=physical_target))],
                lesson_target_ownership_before_decision=copy.deepcopy(before),
                lesson_target_ownership_after_outcome=copy.deepcopy(after),
                physiology={f"{side}_{name}": value for side in ("before", "after")
                            for name, value in dict(energy=.5, hunger=.2, health=1, injury=0, pain=0).items()})


def meal_rows():
    loose = dict(owner=None, consumed=False)
    held = dict(owner=1, consumed=False)
    consumed = dict(owner=None, consumed=True)
    a = row(0, selected=[command("Manipulation", 211, 2)], before=loose, after=held, contact="Touch")
    b = row(1, selected=[command("Manipulation", 210, 2)], before=held, after=consumed, contact="Consumed")
    b["physiology"]["after_energy"] = .6
    return [a, b]


def summary(rows, **kw):
    return care.summarize({"steps": rows}, decision_budget=kw.pop("decision_budget", len(rows)),
                          execution_status=kw.pop("execution_status", "completed"), **kw)


class CareTraceSummaryTests(unittest.TestCase):
    def test_distraction_before_reach_and_visual_reacquisition(self):
        rows = [row(0, selected=[command("Locomotion", 101, 2)]),
                row(1, selected=[command("Locomotion", 101, 3)]),
                row(2, visible=False, selected=[command("Locomotion", 101, 3)]),
                row(3, selected=[command("Locomotion", 101, 2)])]
        original = copy.deepcopy(rows)
        r = summary(rows)
        for name, tick in [("locomotion_switch_away", 1), ("locomotion_switch_back", 3),
                           ("visibility_loss", 2), ("visual_reacquisition", 3)]:
            self.assertEqual(r['metrics'][name]['count'], 1)
            self.assertEqual(r['metrics'][name]['events'][0]['decision_tick'], tick)
        self.assertEqual(r['metrics']['own_pickup']['count'], 0)
        self.assertEqual(r['metrics']['nourishing_consumption']['count'], 0)
        self.assertTrue(r['execution']['budget_exhausted'])
        self.assertEqual(rows, original)

    def test_wrong_entity_eat_with_food_in_reach_never_becomes_a_meal(self):
        # Food candidate is only .6 away, but the actual motor bundle selects another entity.
        r = summary([row(0, selected=[command("Manipulation", 210, 3)], contact="Consumed", physical_target=3)],
                    pilot={'consumed_events': 1, 'food_outcome': {'nourished_meal_events': 99}})
        self.assertEqual(r['metrics']['wrong_entity_eat']['count'], 1)
        self.assertEqual(r['metrics']['nourishing_consumption']['count'], 0)
        self.assertEqual(r['metrics']['exact_target_consumption']['count'], 0)

    def test_pickup_then_exact_nourishing_consumption_from_actual_bundle(self):
        r = summary(meal_rows())
        for name, tick in [('own_pickup', 0), ('exact_target_consumption', 1), ('nourishing_consumption', 1)]:
            self.assertEqual(r['metrics'][name]['count'], 1)
            self.assertEqual(r['metrics'][name]['events'][0]['decision_tick'], tick)
        self.assertFalse(r['neural_execution'])

    def test_eat_can_acquire_loose_food_without_explicit_grab(self):
        rows = meal_rows()
        rows[0]['selected_commands'][0]['primitive'] = 210
        self.assertEqual(summary(rows)['metrics']['own_pickup']['count'], 1)

    def test_consumed_contact_without_body_benefit_or_with_harm_is_not_nourishment(self):
        for changes in [dict(after_energy=.5), dict(after_health=.9), dict(after_injury=.1), dict(after_pain=.1)]:
            with self.subTest(changes=changes):
                rows = meal_rows(); rows[1]['physiology'].update(changes)
                r = summary(rows)
                self.assertEqual(r['metrics']['exact_target_consumption']['count'], 1)
                self.assertEqual(r['metrics']['nourishing_consumption']['count'], 0)

    def test_hunger_reduction_can_establish_measured_benefit(self):
        rows = meal_rows(); rows[1]['physiology'].update(after_energy=.5, after_hunger=.1)
        self.assertEqual(summary(rows)['metrics']['nourishing_consumption']['count'], 1)

    def test_missing_fields_are_unknown_not_representative_or_generic_success(self):
        rows = [dict(tick=0, chosen_action=210, chosen_target=2)]
        r = summary(rows, pilot={'consumed_events': 1})
        for name in ['food_visible', 'own_pickup', 'nourishing_consumption', 'manipulation_switch_away']:
            self.assertIsNone(r['metrics'][name]['count'])
            self.assertEqual(r['metrics'][name]['observed_count'], 0)

    def test_missing_physiology_keeps_consumption_separate_and_nourishment_unknown(self):
        rows = meal_rows(); del rows[1]['physiology']
        r = summary(rows)
        self.assertEqual(r['metrics']['exact_target_consumption']['count'], 1)
        self.assertIsNone(r['metrics']['nourishing_consumption']['count'])

    def test_missing_channel_instrumentation_never_attributes_ownership_transition(self):
        for field in ['selected_commands', 'channel_outcomes', 'outcome_tick']:
            with self.subTest(field=field):
                rows = meal_rows(); del rows[0][field]
                self.assertIsNone(summary(rows)['metrics']['own_pickup']['count'])

    def test_empty_bundle_never_uses_representative_eat_or_unmatched_contact(self):
        rows = meal_rows(); rows[1].update(selected_commands=[], chosen_action=210, chosen_target=2)
        r = summary(rows)
        self.assertEqual(r['metrics']['nourishing_consumption']['observed_count'], 0)
        self.assertIsNone(r['metrics']['nourishing_consumption']['count'])
        rows = meal_rows(); rows[1]['channel_outcomes'][0]['physical']['target_entity'] = 3
        self.assertIsNone(summary(rows)['metrics']['exact_target_consumption']['count'])

    def test_candidate_absence_does_not_hide_tracked_visible_food(self):
        rows = [row(0), row(1)]
        rows[1]['candidates'] = [dict(index=0, action=1, family='Idle', logit=0,
                                      observation='None', target=None, target_position=None)]
        r = summary(rows)
        self.assertEqual(r['metrics']['food_visible']['count'], 2)
        self.assertEqual(r['metrics']['visibility_loss']['count'], 0)

    def test_untargeted_command_does_not_invent_an_entity_switch(self):
        rows = [row(0, selected=[command('Locomotion', 101, 2)]),
                row(1, selected=[command('Locomotion', 105, None)]),
                row(2, selected=[command('Locomotion', 101, 3)])]
        self.assertEqual(summary(rows)['metrics']['locomotion_switch_away']['count'], 0)

    def test_incomplete_timeout_unknown_and_gaps_never_claim_whole_case_counts(self):
        for status in ['incomplete', 'timeout', 'unknown']:
            r = summary(meal_rows(), execution_status=status)
            self.assertIsNone(r['execution']['budget_exhausted'])
            self.assertIsNone(r['metrics']['nourishing_consumption']['count'])
            self.assertEqual(r['metrics']['nourishing_consumption']['observed_count'], 1)
        rows = [row(0, selected=[command('Locomotion', 101, 2)]), row(2, visible=False)]
        r = summary(rows)
        self.assertIsNone(r['metrics']['visibility_loss']['count'])
        self.assertEqual(r['metrics']['visibility_loss']['observed_count'], 0)
        self.assertFalse(r['execution']['contiguous_decisions'])
        self.assertIsNone(summary([], decision_budget=4)['execution']['budget_exhausted'])

    def test_setup_world_tick_offset_is_not_missing_evaluation_capture(self):
        rows = meal_rows()
        for r in rows:
            for key in ['tick', 'decision_tick', 'outcome_tick']:
                r[key] += 130
        r = summary(rows, pilot={'ticks': 2})
        self.assertTrue(r['execution']['contiguous_decisions'])
        self.assertEqual(r['execution']['first_world_tick'], 130)
        self.assertEqual(r['metrics']['nourishing_consumption']['count'], 1)
        self.assertEqual(r['metrics']['nourishing_consumption']['events'][0]['decision_tick'], 131)
        self.assertEqual(r['metrics']['visibility_loss']['count'], 0)

    def test_early_completion_and_unknown_budget_are_not_budget_exhaustion(self):
        self.assertFalse(summary(meal_rows(), decision_budget=128)['execution']['budget_exhausted'])
        self.assertIsNone(summary(meal_rows(), decision_budget=None)['execution']['budget_exhausted'])
        self.assertIsNone(summary(meal_rows(), pilot={'ticks': 3})['metrics']['own_pickup']['count'])
        self.assertIsNone(summary(meal_rows(), decision_budget=1)['metrics']['own_pickup']['count'])

    def test_rejects_ambiguous_target_tracking_duplicate_identity_and_reversed_ticks(self):
        for mutate in [lambda r: r[1].update(lesson_target=3),
                       lambda r: r[1]['object_slots'][0].update(tracked_object_id=10),
                       lambda r: r[1].update(tick=0, decision_tick=0, outcome_tick=1),
                       lambda r: r[1].update(outcome_tick=5),
                       lambda r: r[1]['lesson_target_ownership_after_outcome'].update(owner=1),
                       lambda r: r[1]['selected_commands'].append(command('Manipulation', 210, 2))]:
            rows = meal_rows(); mutate(rows)
            with self.assertRaises(ValueError): summary(rows)

    def test_cli_reads_both_trace_names_without_writing_or_running_native_code(self):
        tool = Path(care.__file__).resolve()
        for name in ['lesson-trace.json', 'lesson-diagnostic.json']:
            with tempfile.TemporaryDirectory() as d:
                p = Path(d); trace = p / name
                trace.write_text(json.dumps({'steps': meal_rows()}), encoding='utf-8')
                before = trace.read_bytes()
                run = subprocess.run([sys.executable, '-B', str(tool), str(p), '--execution-status', 'completed',
                                      '--decision-budget', '128'], capture_output=True, text=True, timeout=10)
                self.assertEqual(run.returncode, 0, run.stderr)
                self.assertEqual(json.loads(run.stdout)['metrics']['nourishing_consumption']['count'], 1)
                self.assertEqual(trace.read_bytes(), before)
                self.assertEqual([x.name for x in p.iterdir()], [name])

    def test_loader_rejects_duplicate_keys_nonfinite_and_oversized_documents(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / 'trace.json'
            for data in ['{"steps":[],"steps":[]}', '{"n":NaN}', '{"n":1e999}']:
                p.write_text(data)
                with self.assertRaises(ValueError): care.load_json(p)
            old = care.MAX_BYTES
            try:
                care.MAX_BYTES = 8; p.write_text('{"steps": []}')
                with self.assertRaisesRegex(ValueError, 'exceeds'): care.load_json(p)
            finally:
                care.MAX_BYTES = old


if __name__ == '__main__':
    unittest.main()
