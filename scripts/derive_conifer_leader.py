"""Derive the audited repair independently from pinned mesh geometry.

Requires NumPy. Apply the same already-chosen 0.8 -> 0.94 endpoint ratio to the
existing trunk's upper cap, after source fitting/LOD generation. Keep the tiny
cap inclination, all X/Z coordinates and every other position. Recalculate only
normals of vertices incident to changed faces, using angle-weighted normals.
This is an isolated post-export recipe, not a claim that a monolithic rebuild
will produce the same LODs or bytes.
"""
import argparse
import json
import struct
from pathlib import Path

import numpy as np

from conifer_glb import Glb
from repair_conifer_leader import BASE_SHA256, FIXED_SHA256, ROOT, sha


def components(positions, triangles):
    unique, inverse = np.unique(positions, axis=0, return_inverse=True)
    parent = np.arange(len(unique))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for a, b, c in inverse[triangles]:
        for u, v in ((a, b), (b, c)):
            u, v = find(u), find(v)
            if u != v:
                parent[v] = u
    labels = np.array([find(i) for i in range(len(unique))])[inverse]
    return [np.where(labels == label)[0] for label in np.unique(labels)]


def changed_normals(old, new, triangles, normals):
    moved = np.any(old != new, axis=1)
    changed_faces = np.any(moved[triangles], axis=1)
    affected = np.unique(triangles[changed_faces])
    result, sums = normals.copy(), np.zeros_like(new)
    vectors = np.cross(new[triangles[:, 1]]-new[triangles[:, 0]], new[triangles[:, 2]]-new[triangles[:, 0]])
    lens = np.linalg.norm(vectors, axis=1)
    if not np.all(lens > 1e-10):
        raise ValueError('Degenerate triangle')
    unit = vectors/lens[:, None]
    for corner in range(3):
        a = new[triangles[:, (corner+1) % 3]]-new[triangles[:, corner]]
        b = new[triangles[:, (corner+2) % 3]]-new[triangles[:, corner]]
        a /= np.linalg.norm(a, axis=1)[:, None]
        b /= np.linalg.norm(b, axis=1)[:, None]
        angle = np.arccos(np.clip(np.sum(a*b, axis=1), -1, 1))
        np.add.at(sums, triangles[:, corner], unit*angle[:, None])
    if not np.all(np.linalg.norm(sums[affected], axis=1) > 1e-10):
        raise ValueError('Undefined changed normal')
    result[affected] = sums[affected]/np.linalg.norm(sums[affected], axis=1)[:, None]
    return result.astype(np.float32), affected


def derive_bytes(raw, source_positions=None):
    if sha(raw) != BASE_SHA256:
        raise ValueError('Baseline SHA256 mismatch')
    base, data, rows = Glb(raw), bytearray(raw), []
    if any(any(k in n for k in ('matrix', 'translation', 'rotation', 'scale')) for n in base.doc['nodes']):
        raise ValueError('Expected identity nodes')
    for level, mesh in enumerate(base.doc['meshes']):
        p = mesh['primitives'][0]
        positions = np.array(base.rows(p['attributes']['POSITION']), dtype=np.float32)
        normals = np.array(base.rows(p['attributes']['NORMAL']), dtype=np.float32)
        triangles = np.array(base.rows(p['indices']), dtype=int).reshape(-1, 3)
        groups = components(positions, triangles)
        trunk = next(g for g in groups if np.argmin(positions[:, 1]) in g)
        caps = trunk[positions[trunk, 1] > 9.4]
        if mesh['name'] != 'Conifer_L'+str(level) or len(caps) != [10, 1, 11][level]:
            raise ValueError('Unexpected trunk cap topology')
        intended = positions.copy()
        shift = float(np.mean(positions[caps, 1]))*(.94/.8-1)
        intended[caps, 1] += shift
        modified = intended if source_positions is None else np.array(source_positions[mesh['name']], dtype=np.float32)
        if not np.array_equal(modified, intended):
            raise ValueError('Editable source differs from this audited repair; review a new repair before shipping')
        updated, affected = changed_normals(positions.astype(float), modified.astype(float), triangles, normals.astype(float))
        for attribute, values, ids in (('POSITION', modified, caps), ('NORMAL', updated, affected)):
            accessor = p['attributes'][attribute]
            for index in ids:
                struct.pack_into('<fff', data, base.offset(accessor, int(index)), *values[index])
        rows.append({'mesh': mesh['name'], 'cap_vertex_ids': caps.tolist(), 'normal_vertex_ids': affected.tolist(),
                     'endpoint_ratio': '.94/.8', 'cap_shift_m': shift})
    if sha(data) != FIXED_SHA256:
        raise ValueError('Derivation did not reproduce the audited candidate hash')
    return bytes(data), rows


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline', type=Path, default=ROOT/'crates/alife_game_app/assets/landscape/island/conifer-leader/Conifer.baseline.glb')
    p.add_argument('--output', type=Path, default=ROOT/'target/artifacts/conifer-leader/Conifer.derived.glb')
    a = p.parse_args()
    if a.baseline.resolve() == a.output.resolve():
        p.error('Baseline and output must differ')
    fixed, rows = derive_bytes(a.baseline.read_bytes())
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_bytes(fixed)
    print(json.dumps({'result': 'PASS', 'numpy': np.__version__, 'sha256': sha(fixed), 'lods': rows}, indent=2))


if __name__ == '__main__':
    main()
