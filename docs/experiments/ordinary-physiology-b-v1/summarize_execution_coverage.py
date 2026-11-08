"""Post-run scalar provenance check. No world execution or private bank export."""
import csv
import json
import sys

study = json.load(open(sys.argv[1]))
rows = []
for trial in study['trials']:
    for interval, frame in enumerate(trial['rows'], 1):
        for decision in frame['decisions']:
            outcomes = decision['outcome'].get('joint', {}).get('channel_outcomes', [])
            meals = [o for o in outcomes if o['physical']['contact'] == 'Consumed']
            representative = decision['candidates'][decision['action']['representative']]
            resting = any(c['primitive'] == 3 for c in decision['selected_bundle']['channels'])
            moved = any(any(o['physical']['displacement'][axis] != 0 for axis in ['x', 'y', 'z']) for o in outcomes)
            before = frame['resident_before']['memory_family_counts'].get('6', 0)
            after = frame['resident_after']['memory_family_counts'].get('6', 0)
            matched = None
            if meals:
                matched = (len(meals) == 1 and meals[0]['channel'] == 'Manipulation'
                           and representative['family'] == 'Ingest' and representative['kind'] == 'Interact'
                           and representative['candidate']['target']['entity'] == meals[0]['physical']['target_entity'])
                assert matched and after - before == 1
            rows.append(dict(**trial['recipe']['arm'], case=trial['recipe']['case']['name'], interval=interval,
                selected_rest=resting, actual_motion=moved,
                body_sleeping_before=frame['biology_before']['body']['sleeping'],
                body_sleeping_after=frame['biology_after']['body']['sleeping'],
                host_sleeping=frame['sleeping_organisms'], host_consolidations=frame['sleep_consolidations'],
                fatigue_delta=frame['biology_after']['homeostasis']['drives']['fatigue']-frame['biology_before']['homeostasis']['drives']['fatigue'],
                consumed_count=len(meals), representative_family=representative['family'],
                representative_kind=representative['kind'], meal_representative_target_matches=matched,
                retained_ingest_before=before, retained_ingest_after=after,
                retained_ingest_delta=after-before))
assert sum(r['consumed_count'] for r in rows) == 19
assert sum(r['selected_rest'] for r in rows) == 43
assert sum(r['body_sleeping_after'] for r in rows) == 11
assert sum(r['selected_rest'] and r['actual_motion'] for r in rows) == 32
assert all(not r['actual_motion'] and r['selected_rest'] for r in rows if r['body_sleeping_after'])
writer = csv.DictWriter(sys.stdout, fieldnames=list(rows[0]), lineterminator='\n')
writer.writeheader()
writer.writerows(rows)
