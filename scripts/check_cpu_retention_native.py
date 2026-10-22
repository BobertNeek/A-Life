#!/usr/bin/env python3
"""Prove native continuation after retiring synthetic historical payloads."""
import argparse
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
from unittest.mock import patch

import run_cpu_care_serial as cpu


def check(binary):
    binary = Path(binary).resolve(strict=True)
    os.environ['ALIFE_SLM_PRIOR'] = 'on'
    os.environ['ALIFE_SLM_PRIOR_BACKEND'] = 'deterministic'
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        source, lane = root / 'initial', root / 'lane'
        cpu.command(binary, '--fixture', '--output', source, '--seed', 9147,
                    '--lesson', 'eat_held_food', '--decisions', 2)
        history, schedule = root / 'history.json', root / 'schedule.json'
        history.write_text(json.dumps({'schema': 1, 'kind': 'mixed-care-seed-history', 'complete': True,
            'evidence': 'Synthetic fixture only; the complete seed history starts in this test.',
            'source_sha256': cpu.pins(source), 'reserved_seed_ranges': [],
            'consumed_seed_ranges': [[9147, 9147]], 'assessment_seeds': [], 'imports': []}))
        schedule.write_text(json.dumps({'schema': 1, 'steps': [
            {'lesson': 'eat_held_food', 'episodes': 5, 'decisions': 2}]}))
        plan = cpu.build_plan(argparse.Namespace(previous=source, lane=lane, binary=binary,
            seed_history=history, schedule=schedule, seeds='9200:9204', temperature=1.,
            known_exclusions=False, keep_checkpoints=2))
        lane.mkdir()
        original_command = cpu.command
        def stop_after_four(binary, *args):
            result = original_command(binary, *args)
            if args[0] == '--previous' and len(cpu.read(lane / cpu.LEDGER)['cpu_care_attempts']) == 4:
                (lane / 'STOP').touch()
            return result
        with patch.object(cpu, 'command', side_effect=stop_after_four):
            try:
                cpu.execute(plan)
            except ValueError as error:
                assert str(error) == 'CPU owner STOP requested interruption' or str(error) == 'CPU owner STOP requested'
            else:
                raise AssertionError('expected deliberate stop before fifth episode')
        before = cpu.command(binary, '--validate-output', lane / 'episode-00003')
        for path in (source, lane / 'episode-00000', lane / 'episode-00001'):
            shutil.rmtree(path)  # Only disposable synthetic fixtures created by this test.
        (lane / 'STOP').unlink()
        result = cpu.execute(plan)
        assert result['accepted_episodes'] == 5
        after = cpu.command(binary, '--validate-output', Path(result['latest_checkpoint']))
        assert after['policy_version'] == before['policy_version'] + 1
        assert after['actor_optimizer_step'] > before['actor_optimizer_step']
        assert after['value_optimizer_step'] > before['value_optimizer_step']
        assert after['objective_state_reset'] is False
        attempts = cpu.read(lane / cpu.LEDGER)['cpu_care_attempts']
        assert [a['seed'] for a in attempts] == list(range(9200, 9205))
        assert all(a['status'] == 'accepted' for a in attempts)
        print('Native continuation passed with old payloads and initial source absent; policy, Adam and seed continuity preserved.')


if __name__ == '__main__':
    check(sys.argv[1])
