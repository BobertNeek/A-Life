"""Safe post-hoc report for the predeclared 28-life gain screen."""
import argparse
import json
from pathlib import Path
from analyze import GENOME, PARENT, analyze_arm, compare, load

LABELS = ['gain-16', 'gain0', 'gain4', 'gain16', 'gain64', 'off', 'mask16']


def label(request):
    if request['arm'] == 'off':
        return 'off'
    if request['arm'] == 'mask':
        return 'mask16'
    return 'gain' + format(request['gain'], 'g')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    groups = {}
    for path in sorted(args.root.iterdir()):
        if not path.is_dir() or not (path / 'REQUEST.json').exists() or not (path / 'COMPLETED.json').exists():
            continue
        request = load(path / 'REQUEST.json')
        assert request['seed'] in [2026111810, 2026111811]
        assert not request['alias'] and request['food_nutrition'] == 0.6
        assert request['gain'] in [-16, 0, 4, 16, 64]
        name = label(request)
        arm, trace = analyze_arm(path)
        arm['request_arm'] = arm['arm']
        arm['arm'] = name
        arms, traces = groups.setdefault((request['seed'], request['reverse_appearance']), ({}, {}))
        assert name not in arms
        arms[name], traces[name] = arm, trace
    cells = []
    for (seed, reverse), (arms, traces) in sorted(groups.items()):
        comparisons = []
        for name in LABELS:
            if name != 'gain0' and name in arms and 'gain0' in arms:
                witness = compare(arms[name], arms['gain0'], traces[name], traces['gain0'])
                if name != 'off':
                    assert all(d['before_first_source'] == 0 for d in witness['differences'].values())
                if name == 'mask16':
                    assert all(d['count'] == 0 for d in witness['differences'].values())
                comparisons.append(witness)
        assert len({a['founder_checkpoint_sha256'] for a in arms.values()}) == 1
        cells.append(dict(seed=seed, reverse_appearance=reverse, alias=False, food_nutrition=0.6,
            complete=set(arms) == set(LABELS), arms=[arms[n] for n in LABELS if n in arms], comparisons=comparisons))
    complete = len(cells) == 4 and all(c['complete'] for c in cells)
    screening = []
    if complete:
        for name in LABELS[:5]:
            losses, selected = [], []
            for cell in cells:
                by_name = {a['arm']: a for a in cell['arms']}
                arm, baseline = by_name[name], by_name['gain0']
                if arm['final_health'] < baseline['final_health'] or arm['meals'] < baseline['meals']:
                    losses.append(dict(seed=cell['seed'], reverse_appearance=cell['reverse_appearance'],
                        meals=arm['meals'], zero_meals=baseline['meals'], health=arm['final_health'], zero_health=baseline['final_health']))
                selected.append(arm)
            screening.append(dict(candidate=name, eligible_without_health_or_per_cell_meal_loss=not losses,
                disqualifying_cells=losses, meals=sum(a['meals'] for a in selected),
                summed_censored_time=sum(a['summed_time_to_meal_or_censor'] for a in selected),
                failed_ingest_containing_bundles=sum(a['failed_ingest_containing_bundles'] for a in selected),
                summed_final_energy=sum(a['final_energy'] for a in selected),
                reserved_work_units=sum(a['reserved_research_work_units'] for a in selected),
                toy_ingest_commands=sum(a['toy_ingest_commands'] for a in selected),
                source_uses=sum(a['source_uses'] for a in selected),
                nonzero_candidate_contributions=sum(a['nonzero_candidate_contributions'] for a in selected)))
    launch = load(args.root / 'LAUNCHED.json')
    result = dict(schema='terminal_gain_screen_safe_v1', complete=complete,
        parent_sha256=PARENT, construction_genome_sha256=GENOME,
        provenance={key: launch[key] for key in ['source_commit', 'host_base', 'binary_sha256', 'profile', 'requests_sha256']},
        expected_runs=28, completed_runs=sum(len(c['arms']) for c in cells), cells=cells, screening=screening,
        reserved_seeds_not_launched=[2026111820, 2026111821],
        selection_scope='exploratory screen; no parent promotion or held-out validation',
        failure_metric='aggregate failed committed bundle containing Ingest; not isolated Ingest-channel failure')
    # Only bounded scalars, receipts, hashes and counters leave the LOCAL traces.
    def safe(value):
        if isinstance(value, dict):
            assert not set(value) & {'patch', 'initial', 'final_features', 'replay_tick', 'neural', 'residents', 'weights', 'first_moment', 'second_moment', 'update_ages', 'query_feature_bits'}
            for child in value.values(): safe(child)
        elif isinstance(value, list):
            assert len(value) <= 64
            for child in value: safe(child)
    safe(result)
    with args.output.open('x') as out:
        json.dump(result, out, indent=2)
        out.write('\n')


if __name__ == '__main__':
    main()
