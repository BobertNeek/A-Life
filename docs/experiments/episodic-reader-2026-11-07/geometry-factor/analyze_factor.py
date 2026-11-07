#!/usr/bin/env python3
"""Audit completed fixed-brain/world-geometry journals without rerunning lives."""
import collections
import hashlib
import json
import math
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent
ARCHIVE = ROOT.parent / 'v8-contact-compensation'
SHA = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def analyze():
    manifest = json.loads((ROOT / 'run-manifest.json').read_text())
    assert SHA(ROOT / 'source.rs') == manifest['harness_sha256']
    assert SHA(ROOT / 'applied-geometry.patch') == manifest['geometry_patch_sha256']
    for key in ['old_geometry_executable', 'new_geometry_executable']:
        entry = manifest[key]
        assert SHA(ROOT.parent / entry['path']) == entry['sha256']
    expected = {(seed, scene, arm)
                for seed in range(2026111101, 2026111105)
                for scene in ['obstacle-food', 'far-food']
                for arm in ['whole-care-v5', 'whole-care']}
    worlds = {}
    hashes = {}
    groups = collections.defaultdict(list)
    for world in ['old-geometry', 'new-geometry']:
        path = ROOT / world
        journal = json.loads((path / 'receipt.json').read_text())
        assert journal['complete'] and journal['receipt_format'] == 'journal-v2'
        assert journal['ticks'] == 120 and math.isclose(journal['initial_energy'], 0.4, rel_tol=0., abs_tol=1e-7)
        assert journal['preparation_ticks'] == 0 and journal['target_hazard_pain'] == 0.
        assert len(journal['trial_files']) == len(set(journal['trial_files'])) == 16
        trials = {}
        for name in journal['trial_files']:
            assert pathlib.Path(name).name == name
            trial_path = path / name
            trial = json.loads(trial_path.read_text())
            key = (trial['seed'], trial['scene'], trial['arm'])
            assert key not in trials
            trials[key] = trial
            hashes[f'{world}/{name}'] = SHA(trial_path)
            assert len(trial['rows']) == 120
            assert [r['tick'] for r in trial['rows']] == list(range(120))
            assert trial['optimizer_unchanged'] and trial['cold_next_step_exact']
            assert trial['final_body']['injury'] == 0.
            assert trial['final_body']['health'] == 1.
            target_entity = 3 if trial['scene'] == 'obstacle-food' else 2
            # Scene construction, not a semantic tag inferred from action success.
            assert any(c['target']['entity'] == target_entity for c in trial['rows'][0]['candidates'])
            consumption = []
            touches = []
            movement = collections.Counter()
            movement_targets = collections.Counter()
            path_length = 0.
            for row in trial['rows']:
                for outcome in row['outcome']['joint']['channel_outcomes']:
                    physical = outcome['physical']
                    if physical['target_entity'] == target_entity:
                        if physical['contact'] == 'Consumed':
                            consumption.append(row['tick'])
                        if physical['contact'] == 'Touch':
                            touches.append(row['tick'])
                    if outcome['channel'] == 'Locomotion':
                        movement[physical['contact']] += 1
                        movement_targets[str(physical['target_entity'])] += 1
                        path_length += math.sqrt(sum(v*v for v in physical['displacement'].values()))
            assert (not trial['target_active']) == bool(consumption)
            final = trial['final_position']
            groups[(world, trial['scene'], trial['arm'])].append({
                'seed':trial['seed'],
                'target_entity':target_entity,
                'consumed':bool(consumption),
                'consumed_ticks':consumption,
                'first_target_touch_tick':min(touches,default=None),
                'movement_outcomes':dict(movement),
                'movement_receipt_targets':dict(movement_targets),
                'path_length':path_length,
                'final_position':final,
                'final_distance_to_initial_food':math.sqrt((final['x']-2.5)**2+final['y']**2+final['z']**2),
                'final_energy':trial['final_body']['energy'],
                'final_injury':trial['final_body']['injury'],
                'final_target_carrier':trial['final_target_carrier'],
                'candidate_digest':trial['candidate_digest'],
            })
        assert set(trials) == expected
        for seed in range(2026111101, 2026111105):
            for scene in ['obstacle-food', 'far-food']:
                assert trials[(seed,scene,'whole-care-v5')]['initial_biology'] == trials[(seed,scene,'whole-care')]['initial_biology']
        worlds[world] = trials
    old = worlds['old-geometry']
    new = worlds['new-geometry']
    old_reproduced = []
    for key in sorted(expected):
        assert old[key]['initial_biology'] == new[key]['initial_biology']
        assert old[key]['candidate_digest'] == new[key]['candidate_digest']
        seed, scene, arm = key
        asset = f'{seed}.{arm}.alife-foundation'
        a = (ROOT/'old-geometry'/asset).read_bytes()
        b = (ROOT/'new-geometry'/asset).read_bytes()
        assert a == b == (ARCHIVE/'held-out'/asset).read_bytes()
        if scene == 'obstacle-food':
            archived = json.loads((ARCHIVE/'held-out'/f'{seed}.{scene}.{arm}.json').read_text())
            assert old[key]['rows'] == archived['rows']
            assert old[key]['final_body'] == archived['final_body']
            old_reproduced.append(f'{seed}.{scene}.{arm}')
    return {
        'manifest_sha256':SHA(ROOT/'run-manifest.json'),
        'trial_count':sum(len(t) for t in worlds.values()),
        'requested_rows':3840,
        'coverage_exact':True,
        'candidate_assets_exact_across_worlds_and_archive':True,
        'paired_initial_biology_exact':True,
        'optimizer_and_full_cold_continuation_all_pass':True,
        'old_geometry_obstacle_rows_reproduce_archived_trials':old_reproduced,
        'trial_sha256':hashes,
        'groups':[
            {'world':world,'scene':scene,'arm':arm,'n':len(trials),
             'consumed':sum(t['consumed'] for t in trials),'trials':trials}
            for (world,scene,arm),trials in sorted(groups.items())
        ],
    }


if __name__ == '__main__':
    print(json.dumps(analyze(),indent=2,allow_nan=False))
