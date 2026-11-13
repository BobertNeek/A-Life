#!/usr/bin/env python3
"""Frozen-state decoder sensitivity; not a prediction of whole-life behavior."""
import argparse
import json
import pathlib
import torch
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('trace', type=pathlib.Path)
p.add_argument('output', type=pathlib.Path)
a = p.parse_args()
rows = []
# Additional edits relative to the accepted pursuit-memory16 parent.
direction = torch.tensor([-3.5628414154052734, 49.11018347740173, 10.744718551635742, 4.])
for trial in json.loads(a.trace.read_text())['trials']:
    for row in trial['trace'][::32]:
        for i, c in enumerate(row['candidates']):
            if c['family'] != 'Approach' or c['target']['entity'] is None:
                continue
            f = torch.tensor(c['features'])
            m = torch.tensor(row['motor_activations'])
            sensitivity = torch.stack((m[2] * f[6:15].sum(), m[2] * f[2],
                                       m[3] * f[18], m[3] * f[1]))
            rows.append({'scene': trial['scene'], 'tick': row['tick'], 'target': c['target']['entity'],
                         'old_logit': row['logits'][i],
                         'sensitivity': sensitivity.tolist(),
                         'predicted_new_logit_on_frozen_old_state': row['logits'][i] + float(direction @ sensitivity)})
with a.output.open('x') as f:
    json.dump({'method': __doc__, 'torch': torch.__version__, 'rows': rows}, f, indent=2)
