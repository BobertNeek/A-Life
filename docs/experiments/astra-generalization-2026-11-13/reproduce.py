#!/usr/bin/env python3
"""Replay one archived native request. Supply the matching source asset/compiler."""
import argparse
import gzip
import json
import pathlib
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('record')
p.add_argument('binary', type=pathlib.Path)
p.add_argument('foundation', type=pathlib.Path)
p.add_argument('--output', type=pathlib.Path, required=True)
p.add_argument('--trace', action='store_true')
a = p.parse_args()
with gzip.open(pathlib.Path(__file__).with_name('results.jsonl.gz'), 'rt') as f:
    records = (json.loads(line) for line in f)
    record = next((r for r in records if r['name'] == a.record), None)
if record is None:
    p.error('unknown record')
request = dict(record['request'])
request.pop('export', None)
# The archive retains behavioral metrics; full neural traces can be requested separately.
request['trace'] = a.trace
result = subprocess.run([str(a.binary.resolve()), str(a.foundation.resolve())],
                        input=json.dumps(request) + '\n', text=True, capture_output=True, check=True)
value = json.loads(result.stdout)
if 'error' in value:
    raise RuntimeError(value['error'])
if value['base_digest'] != record['result']['base_digest']:
    raise RuntimeError('wrong foundation: source digest differs')
with a.output.open('x') as f:
    json.dump(value, f, separators=(',', ':'))
def outcomes(v):
    return [{k: x for k, x in t.items() if k not in ('trace', 'target_motion')} for t in v['trials']]
expected = outcomes(record['result'])
actual = outcomes(value)
metrics_match = len(actual) == len(expected) and all(
    all(k in got and got[k] == x for k, x in want.items())
    for got, want in zip(actual, expected))
digest_matches = value['candidate_digest'] == record['result']['candidate_digest']
print(json.dumps({'candidate_digest_matches': digest_matches,
                  'trial_metrics_match': metrics_match,
                  'seconds': value['seconds'], 'output': str(a.output)}))
if not digest_matches or not metrics_match:
    raise SystemExit(1)
