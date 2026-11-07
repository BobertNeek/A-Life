"""Stdlib-only SYNTHETIC reporting tests. These do not run or qualify A-Life.

python3 -m unittest discover -s docs/experiments/episodic-reader-2026-11-07 \
    -p test_transfer_reporting.py -v
"""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import transfer_reporting as reporting


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def arm_spec(name):
    return dict(name=name, action_gain=0.25, memory_reader=True, memory_gain=12.0,
                family_valence_reader=name == "family-value-on", filename=name + ".alife-foundation",
                sha256=reporting.ASSETS[name])


def manifest():
    return dict(schema="fresh-native-transfer-v1", source_commit="a" * 40,
                executable_sha256="b" * 64, retrieval_source_sha256="c" * 64,
                plan=dict(construction_seed=reporting.CONSTRUCTION_SEED, max_lives=8,
                          episodes=[dict(seed=seed, mirror_z=mirror) for seed, mirror in reporting.EPISODES],
                          asset_sha256=list(reporting.ASSETS.values())), spec=copy.deepcopy(reporting.SPEC))


def target(label, identity, mirror):
    z = -0.35 if label == "A" else 0.35
    return dict(label=label, id=identity, kind="Food", consumed=False, carried_by=None,
                position=dict(x=0.35, y=0.0, z=-z if mirror else z), nutrition=0.05,
                hazard_pain=0.35 if label == "A" else 0.0)


def make_candidate(index, family, kind, action, entity):
    features = [0.0] * 24
    features[18] = 1.0 if entity is not None else 0.0
    return dict(candidate_index=index, family=family, kind=kind, action_id=action,
                target=dict(entity=entity), features=features)


def receipt(candidates, logits, seed, counter, channels=None):
    representative, motor, masks, forced = reporting.sampled_indices(candidates, logits, seed, counter, channels or ["Locomotion"])
    return dict(candidate_count=len(candidates), representative_mask=(1 << len(candidates)) - 1,
                motor_masks=masks, forced_slots=forced, representative=representative,
                motor_candidates=motor, temperature=1.0, old_joint_log_probability=-0.1)


def synthetic_life(seed, arm, mirror, a_id=41, b_id=99, reverse_targets=False):
    """Three fabricated rows: grab A, fail Ingest(B) while holding A, consume A.

    This exercises data reduction, not validity of a real organism trajectory.
    """
    m = manifest()
    targets = [target("A", a_id, mirror), target("B", b_id, mirror)]
    if reverse_targets:
        targets.reverse()
    start = dict(event="start", scene="learning", arm=arm_spec(arm), spec=copy.deepcopy(reporting.SPEC),
                 backend="native-cpu", experiment="family-value", retrieval_source_sha256=m["retrieval_source_sha256"],
                 fresh_disposable=True, no_adam_updates=True, no_learning_off_comparator=True,
                 episode=dict(seed=seed, mirror_z=mirror), construction_seed=reporting.CONSTRUCTION_SEED,
                 motor_channels=["Locomotion"],
                 target_objects=targets, initial_biology=dict(body=dict(health=1.0, injury=0.0, energy=0.4)))
    candidates = [make_candidate(1, "Ingest", "Interact", 210, a_id),
                  make_candidate(2, "Ingest", "Interact", 210, b_id),
                  make_candidate(3, "Contact", "Interact", 211, a_id),
                  make_candidate(4, "Avoid", "Move", 102, a_id),
                  make_candidate(5, "Idle", "Idle", 1, None)]
    events = [start]
    for index, choice in enumerate((2, 1, 0)):
        logits = [-60.0] * len(candidates)
        logits[choice] = 60.0
        action = receipt(candidates, logits, seed, index)
        selected = candidates[action["representative"]]
        counter_logits = list(logits)
        if arm == "family-value-on" and index == 1:
            counter_logits[0] = 80.0
        physical = dict(contact="Consumed" if index == 2 else "Touch" if index == 0 else "Blocked",
                        target_entity=a_id if index != 1 else b_id)
        events.append(dict(event="decision", index=index, tick=600 + index, sampling_counter=index,
                           action=action, selected_command=dict(action_id=selected["action_id"],
                           target_entity=selected["target"]["entity"], organism_id=7),
                           candidates=copy.deepcopy(candidates), logits=logits,
                           outcome=dict(physical=physical, organism_id=7, success=index != 1),
                           biological_reward=-0.3 if index == 2 else 0.0,
                           aversive_harm=0.35 if index == 2 else 0.0,
                           experienced_valence=-0.2 if index == 2 else 0.0,
                           consumed_target_indices=[next(i for i, obj in enumerate(targets) if obj["id"] == a_id)] if index == 2 else [],
                           target_margins=reporting.margins(candidates, logits, {"A": a_id, "B": b_id}),
                           detached_assay=dict(uncompressed_provenance_sha256="d" * 64,
                           actual_pre_update=reporting.margins(candidates, logits, {"A": a_id, "B": b_id}),
                           without_ingest_family_valence_context=reporting.margins(candidates, counter_logits, {"A": a_id, "B": b_id}),
                           without_ingest_family_valence_choice_same_counter=receipt(candidates, counter_logits, seed, index)),
                           memory_and_cognitive_inputs=[["SYNTHETIC_PRIVATE_SENTINEL"]],
                           watched_readouts={"private_payload": "SYNTHETIC_PRIVATE_SENTINEL"}))
        boundary_targets = copy.deepcopy(targets)
        for obj in boundary_targets:
            if obj["label"] == "A":
                obj["carried_by"] = 7 if index < 2 else None
                obj["consumed"] = index == 2
        events.append(dict(event="boundary", index=index, tick=601 + index, consolidations=0,
                           targets=boundary_targets, body=dict(health=0.65 if index == 2 else 1.0, injury=0.0, energy=0.4)))
    events.append(dict(event="requested_recovery_sleep", tick=603, reason="SYNTHETIC_PRIVATE_SENTINEL"))
    summary = dict(schema="fresh-native-life-result-v1", complete=True, backend="native-cpu", scene="learning",
                   arm=arm_spec(arm), source_commit=m["source_commit"], experiment="family-value",
                   retrieval_source_sha256=m["retrieval_source_sha256"], seed=seed,
                   episode=dict(seed=seed, mirror_z=mirror), construction_seed=reporting.CONSTRUCTION_SEED,
                   steps=3, decisions=3, stop_reason="vitality_safeguard", consumptions=[0, 1] if reverse_targets else [1, 0],
                   negative_a_meals=1, negative_a_experienced_valence_meals=1,
                   a_ingest_opportunities=3, paired_ingest_opportunities=3,
                   a_contact_ingest_opportunities=3, paired_contact_ingest_opportunities=3,
                   requested_sleep_tick=603, sleep_consolidations=0, post_sleep_decisions=0,
                   final_body=dict(health=0.65, injury=0.0, energy=0.4), optimizer_exact=True,
                   cold_restore=dict(checkpoint_roundtrip_exact=True, next_checkpoint_exact=True,
                                     next_step=dict(completed=True, decisions=1)),
                   phase_provenance=dict(meals_checked=1, all_consumed_known=True),
                   reversal_tested=False, unrelated_acquired_retention_tested=False,
                   diagnostic_uncompressed_sha256="SYNTHETIC_PRIVATE_SENTINEL")
    return events, summary


class SyntheticReportingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="synthetic-transfer-reporting-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.private, self.public = self.base / "PRIVATE", self.base / "PUBLIC"
        self.private.mkdir()
        write_json(self.private / "manifest.json", manifest())

    def add_life(self, assignment=reporting.ASSIGNMENTS[0], **kwargs):
        seed, arm, mirror = assignment
        events, summary = synthetic_life(seed, arm, mirror, **kwargs)
        folder = self.private / f"episode-{seed}"
        folder.mkdir(exist_ok=True)
        journal = folder / f"{arm}.learning.jsonl"
        summary_path = folder / f"{arm}.learning.summary.json"
        journal.write_text("\n".join(json.dumps(row) for row in events) + "\n", encoding="utf-8")
        write_json(summary_path, summary)
        return journal, summary_path

    def complete(self):
        results = []
        for assignment in reporting.ASSIGNMENTS:
            _, path = self.add_life(assignment)
            results.append(reporting.read_json(path))
        write_json(self.private / "transfer.complete.json", dict(complete=True, results=results))

    def mutate_summary(self, fn):
        _, path = self.add_life()
        value = reporting.read_json(path)
        fn(value)
        write_json(path, value)

    def mutate_journal(self, fn):
        journal, _ = self.add_life()
        events = [json.loads(line) for line in journal.read_text().splitlines()]
        fn(events)
        journal.write_text("\n".join(json.dumps(event) for event in events) + "\n")

    def test_synthetic_eight_lives_are_complete_and_sanitized(self):
        self.complete()
        self.assertEqual(reporting.publish(self.private, self.public, require_complete=True), 0)
        summary = reporting.read_json(self.public / "summary.json")
        causal = reporting.read_json(self.public / "causal-rows.json")
        reporting.validate_public(summary, causal)
        self.assertEqual(reporting.validate_public_outputs(self.public), (summary, causal))
        self.assertEqual(summary["status"], "complete")
        self.assertEqual(summary["completed_lives"], 8)
        self.assertEqual(len(causal["rows"]), 24)
        counts = summary["lives"][0]["counts"]
        self.assertEqual(counts["held_a_b_ingest_failures"], 1)
        self.assertEqual(counts["held_a_release_opportunities"], 2)
        self.assertEqual(counts["negative_a_biological_credit"], 1)
        self.assertEqual(summary["lives"][1]["counts"]["family_ablation_to_ingest_a"], 1)
        self.assertEqual({p.name for p in self.public.iterdir()}, set(reporting.PUBLIC_LIMITS))
        for file in self.public.iterdir():
            self.assertNotIn("SYNTHETIC_PRIVATE_SENTINEL", file.read_text())
            self.assertLessEqual(file.stat().st_size, reporting.PUBLIC_LIMITS[file.name])

    def test_synthetic_partial_is_saved_before_strict_exit(self):
        self.add_life()
        self.assertEqual(reporting.publish(self.private, self.public, require_complete=True), 2)
        summary = reporting.read_json(self.public / "summary.json")
        self.assertEqual(summary["status"], "partial")
        self.assertEqual(summary["completed_lives"], 1)
        self.assertEqual(len(summary["missing_assignments"]), 7)

    def test_eight_summaries_without_completion_marker_remain_partial(self):
        for assignment in reporting.ASSIGNMENTS:
            self.add_life(assignment)
        summary, _ = reporting.collect(self.private)
        self.assertEqual(summary["status"], "partial")
        self.assertFalse(summary["completion_marker_present"])

    def test_missing_summary_is_not_fabricated_from_unfinished_journal(self):
        journal, _ = self.add_life()
        folder = self.private / "episode-2026111202"
        folder.mkdir()
        (folder / "family-value-on.learning.jsonl").write_text(journal.read_text())
        summary, _ = reporting.collect(self.private)
        self.assertEqual(summary["completed_lives"], 1)

    def test_not_started_requires_explicit_opt_in_and_has_no_provenance(self):
        absent = self.base / "NOT_CREATED"
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(absent)
        self.assertEqual(reporting.publish(absent, self.public, True, True), 2)
        summary = reporting.read_json(self.public / "summary.json")
        self.assertEqual(summary["status"], "not_started")
        self.assertIsNone(summary["source_commit"])
        self.assertEqual(len(summary["missing_assignments"]), 8)

    def test_not_started_rejects_existing_life_files_without_manifest(self):
        unsafe = self.base / "UNBOUND"
        unsafe.mkdir()
        (unsafe / "life.jsonl").write_text("{}")
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(unsafe, allow_not_started=True)

    def test_non_eight_completion_and_duplicates_rejected(self):
        self.complete()
        path = self.private / "transfer.complete.json"
        original = reporting.read_json(path)
        for results in (original["results"][:-1], original["results"] + original["results"][:1],
                        original["results"][:-1] + original["results"][:1]):
            with self.subTest(length=len(results)):
                write_json(path, dict(complete=True, results=results))
                with self.assertRaises(reporting.ValidationError):
                    reporting.collect(self.private)

    def test_unassigned_or_duplicate_summary_location_rejected(self):
        _, path = self.add_life()
        write_json(self.private / "extra" / "duplicate.summary.json", reporting.read_json(path))
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)

    def test_wrong_asset_seed_and_budget_rejected(self):
        self.add_life()
        path = self.private / "manifest.json"
        for mutate in (lambda value: value["plan"]["asset_sha256"].__setitem__(0, "0" * 64),
                       lambda value: value["plan"]["episodes"][0].__setitem__("seed", 2026111209),
                       lambda value: value["spec"].__setitem__("life_ticks", 257),
                       lambda value: value["spec"].__setitem__("max_a_consumptions", 4),
                       lambda value: value["spec"].__setitem__("preparation_ticks", 0),
                       lambda value: value["plan"].__setitem__("max_lives", 9)):
            value = manifest()
            mutate(value)
            write_json(path, value)
            with self.assertRaises(reporting.ValidationError):
                reporting.collect(self.private)

    def test_wrong_life_seed_or_asset_rejected(self):
        for key, value in (("seed", 2026110901), ("construction_seed", 2026111201)):
            self.mutate_summary(lambda summary: summary.__setitem__(key, value))
            with self.assertRaises(reporting.ValidationError):
                reporting.collect(self.private)
        self.mutate_summary(lambda summary: summary["arm"].__setitem__("sha256", "0" * 64))
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)

    def test_target_ids_and_target_order_are_resolved_not_hardcoded(self):
        self.add_life(a_id=9001, b_id=6, reverse_targets=True)
        summary, _ = reporting.collect(self.private)
        counts = summary["lives"][0]["counts"]
        self.assertEqual((counts["a_meals"], counts["b_meals"]), (1, 0))
        self.assertEqual(counts["held_a_b_ingest_failures"], 1)

    def test_consumed_target_mapping_bug_rejected(self):
        self.mutate_journal(lambda events: events[5].__setitem__("consumed_target_indices", [1]))
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)

    def test_alias_and_changed_target_identity_rejected(self):
        self.mutate_journal(lambda events: events[0]["target_objects"][1].__setitem__("id", 41))
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)
        self.mutate_journal(lambda events: events[2]["targets"][1].__setitem__("id", 1000))
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)

    def test_false_preservation_phase_and_retention_flags_rejected(self):
        mutations = (lambda s: s.__setitem__("optimizer_exact", False),
                     lambda s: s["cold_restore"].__setitem__("checkpoint_roundtrip_exact", False),
                     lambda s: s["cold_restore"].__setitem__("next_checkpoint_exact", False),
                     lambda s: s["phase_provenance"].__setitem__("all_consumed_known", False),
                     lambda s: s["phase_provenance"].__setitem__("meals_checked", 0),
                     lambda s: s.__setitem__("unrelated_acquired_retention_tested", True))
        for mutation in mutations:
            self.mutate_summary(mutation)
            with self.assertRaises(reporting.ValidationError):
                reporting.collect(self.private)

    def test_actual_sampling_margin_and_causal_choice_tampering_rejected(self):
        mutations = (lambda rows: rows[1]["action"].__setitem__("representative", 0),
                     lambda rows: rows[1]["target_margins"].__setitem__("ingest_a", 1000.0),
                     lambda rows: rows[1]["detached_assay"]["without_ingest_family_valence_choice_same_counter"].__setitem__("representative", 0),
                     lambda rows: rows[1]["detached_assay"]["actual_pre_update"].__setitem__("ingest_b", None))
        for mutation in mutations:
            self.mutate_journal(mutation)
            with self.assertRaises(reporting.ValidationError):
                reporting.collect(self.private)

    def test_counters_stop_and_sleep_timing_are_checked(self):
        for mutation in (lambda s: s.__setitem__("negative_a_meals", 0),
                         lambda s: s.__setitem__("a_contact_ingest_opportunities", 0),
                         lambda s: s.__setitem__("stop_reason", "tick_budget"),
                         lambda s: s.__setitem__("stop_reason", "harmful_consumption_cap"),
                         lambda s: s.__setitem__("requested_sleep_tick", None)):
            self.mutate_summary(mutation)
            with self.assertRaises(reporting.ValidationError):
                reporting.collect(self.private)
        self.mutate_journal(lambda rows: rows[-1].__setitem__("tick", 602))
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)

    def test_public_recursive_allowlists_reject_extra_fields_and_private_payload(self):
        self.add_life()
        summary, causal = reporting.collect(self.private)
        for location in ((), ("lives", 0), ("lives", 0, "counts")):
            changed = copy.deepcopy(summary)
            obj = changed
            for part in location:
                obj = obj[part]
            obj["neural_bank"] = [1, 2, 3]
            with self.assertRaises(reporting.ValidationError):
                reporting.validate_public(changed, causal)
        for location in ((), ("rows", 0), ("rows", 0, "actual_scores")):
            changed = copy.deepcopy(causal)
            obj = changed
            for part in location:
                obj = obj[part]
            obj["source_row"] = {"private_payload": "secret"}
            with self.assertRaises(reporting.ValidationError):
                reporting.validate_public(summary, changed)
        changed = copy.deepcopy(summary)
        changed["source_commit"] = "PRIVATE/path-or-payload"
        with self.assertRaises(reporting.ValidationError):
            reporting.validate_public(changed, causal)

    def test_public_false_complete_or_missing_coverage_rejected(self):
        self.add_life()
        summary, causal = reporting.collect(self.private)
        changed = copy.deepcopy(summary)
        changed["status"] = "complete"
        with self.assertRaises(reporting.ValidationError):
            reporting.validate_public(changed, causal)
        changed = copy.deepcopy(summary)
        changed["missing_assignments"].pop()
        with self.assertRaises(reporting.ValidationError):
            reporting.validate_public(changed, causal)

    def test_duplicate_keys_nonfinite_and_oversize_inputs_rejected(self):
        for data in ('{"x":1,"x":2}', '{"x":NaN}', '{"x":Infinity}'):
            with self.assertRaises(reporting.ValidationError):
                reporting.decode(data)
        self.add_life()
        with mock.patch.object(reporting, "JOURNAL_LIMIT", 20):
            with self.assertRaises(reporting.ValidationError):
                reporting.collect(self.private)
        with mock.patch.dict(reporting.PUBLIC_LIMITS, {"summary.json": 10}):
            with self.assertRaises(reporting.ValidationError):
                reporting.publish(self.private, self.public)
        self.assertFalse(self.public.exists())

    def test_private_diagnostics_are_never_opened(self):
        self.add_life()
        path = self.private / "episode-2026111201" / "private.fresh-diagnostic.json.zst"
        path.write_text("SYNTHETIC_PRIVATE_SENTINEL")
        original = Path.open
        opened = []

        def guarded_open(path, *args, **kwargs):
            self.assertNotEqual(path.suffix, ".zst")
            opened.append(path)
            return original(path, *args, **kwargs)

        with mock.patch.object(Path, "open", guarded_open):
            reporting.publish(self.private, self.public)
        self.assertNotIn(path, opened)

    def test_invalid_later_input_does_not_overwrite_valid_partial_outputs(self):
        self.add_life()
        reporting.publish(self.private, self.public)
        before = {path.name: path.read_bytes() for path in self.public.iterdir()}
        self.mutate_summary(lambda summary: summary.__setitem__("optimizer_exact", False))
        self.assertEqual(reporting.main([str(self.private), str(self.public)]), 1)
        self.assertEqual(before, {path.name: path.read_bytes() for path in self.public.iterdir()})

    def test_dynamic_native_motor_channel_prefix_and_masks(self):
        journal, _ = self.add_life()
        rows = [json.loads(line) for line in journal.read_text().splitlines()]
        channels = ["Locomotion", "Manipulation"]
        rows[0]["motor_channels"] = channels
        for row in rows:
            if row["event"] != "decision":
                continue
            row["action"] = receipt(row["candidates"], row["logits"], rows[0]["episode"]["seed"], row["sampling_counter"], channels)
            row["detached_assay"]["without_ingest_family_valence_choice_same_counter"] = copy.deepcopy(row["action"])
        journal.write_text("\n".join(json.dumps(row) for row in rows) + "\n")
        summary, _ = reporting.collect(self.private)
        self.assertEqual(summary["completed_lives"], 1)
        rows[0]["motor_channels"] = ["Manipulation"]
        journal.write_text("\n".join(json.dumps(row) for row in rows) + "\n")
        with self.assertRaises(reporting.ValidationError):
            reporting.collect(self.private)

    def test_public_report_tampering_and_oversize_rejected_on_reopen(self):
        self.add_life()
        reporting.publish(self.private, self.public)
        report = self.public / "report.md"
        report.write_text(report.read_text() + "SYNTHETIC_PRIVATE_SENTINEL")
        with self.assertRaises(reporting.ValidationError):
            reporting.validate_public_outputs(self.public)
        reporting.publish(self.private, self.public)
        with mock.patch.dict(reporting.PUBLIC_LIMITS, {"causal-rows.json": 5}):
            with self.assertRaises(reporting.ValidationError):
                reporting.validate_public_outputs(self.public)

    def test_reader_off_can_acquire_a_family_lane_contribution(self):
        journal, _ = self.add_life()
        rows = [json.loads(line) for line in journal.read_text().splitlines()]
        row = rows[3]
        logits = list(row["logits"])
        logits[0] = 80.0
        row["detached_assay"]["without_ingest_family_valence_context"] = reporting.margins(
            row["candidates"], logits, {"A": 41, "B": 99})
        row["detached_assay"]["without_ingest_family_valence_choice_same_counter"] = receipt(
            row["candidates"], logits, rows[0]["episode"]["seed"], row["sampling_counter"])
        journal.write_text("\n".join(json.dumps(row) for row in rows) + "\n")
        summary, _ = reporting.collect(self.private)
        self.assertEqual(summary["lives"][0]["counts"]["family_ablation_to_ingest_a"], 1)


if __name__ == "__main__":
    unittest.main()
