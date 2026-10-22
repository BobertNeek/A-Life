#!/usr/bin/env python3
"""Describe payloads eligible for retirement; never deletes or changes a lane.

The updated scheduler must successfully resume/validate its saved recent-checkpoint
plan before applying a report. Preserve the plan's starting source, all receipts,
seed/history metadata, and separately owned original creature archives.
"""
import argparse
import json
from pathlib import Path

import run_cpu_care_serial as cpu


def report(lane):
    lane = Path(lane).resolve(strict=True)
    plan = cpu.read(lane / cpu.PLAN)
    policy = plan.get('retention', {})
    cpu.require(policy.get('mode') == 'recent-checkpoints' and
                type(policy.get('keep')) is int and policy['keep'] >= 2,
                'a saved bounded-retention plan is required; never silently migrate an old plan')
    attempts = cpu.read(lane / cpu.LEDGER)['cpu_care_attempts']
    cpu.require(not any(a['status'] == 'pending' for a in attempts), 'reconcile pending work first')
    accepted = [a for a in attempts if a['status'] == 'accepted']
    candidates = []
    for attempt in accepted[:-policy['keep']]:
        directory = Path(attempt['output'])
        cpu.require(directory.parent.resolve() == lane and not directory.is_symlink(),
                    'retirement target must be a direct owned lane directory')
        if not directory.exists():
            continue
        files = []
        for path in directory.rglob('*'):
            cpu.require(not path.is_symlink(), 'unexpected symlink in retirement candidate')
            if path.is_file() and path.name != 'cpu-cycle.json':
                files.append({'path': str(path), 'logical_bytes': path.stat().st_size})
        candidates.append({'episode': str(directory), 'files': files,
                           'logical_bytes': sum(f['logical_bytes'] for f in files)})
    return {'action': 'report-only', 'retained_checkpoints': [a['output'] for a in accepted[-policy['keep']:]],
            'starting_source': plan['source'],
            'ledger_sha256': cpu.mixed.host.sha256(lane / cpu.LEDGER),
            'plan_sha256': cpu.mixed.host.sha256(lane / cpu.PLAN), 'retirement_candidates': candidates,
            'candidate_logical_bytes': sum(c['logical_bytes'] for c in candidates),
            'physical_savings_measured': False, 'files_deleted': 0}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', type=Path)
    print(json.dumps(report(parser.parse_args().lane), indent=2))
