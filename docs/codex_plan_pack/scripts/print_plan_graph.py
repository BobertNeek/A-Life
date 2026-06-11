#!/usr/bin/env python3
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / 'plan_manifest.json').read_text())
completed = set()
status = root / 'completed_plans.txt'
if status.exists():
    completed = {line.strip() for line in status.read_text().splitlines() if line.strip()}
print('Completed:', ', '.join(sorted(completed)) or '(none)')
print('Unblocked:')
for p in manifest['plans']:
    if p['id'] in completed:
        continue
    if all(dep in completed for dep in p['dependencies']):
        print(f"- {p['id']}: {p['title']} [{p['branch']}] -> {', '.join(p['next']) if p['next'] else 'final'}")
