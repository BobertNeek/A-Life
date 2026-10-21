"""Reviewed, instance-specific island authoring corrections (metres, Y-up).

Do not ground the rock library globally: its authored elevations are intentional.
The sampled near terrain differs from the analytic authoring function here.
Each target has independent triangle-contact and exposed-gap evidence. Heights
embed the relevant rock LODs by 2 cm plus their reviewed support-height spread.
BoulderA 1325 supports visible L0/L1 only: its approved L2 simplification would
require overburying those meshes. Current production culling never displays
non-tree L2.
"""

CORRECTIONS = {
    1319: {
        'kind': 'BoulderC',
        'xz': (35.826016966114025, -212.13306068550276),
        'original_y': 155.7075371101759,
        'corrected_y': 154.78496851454122,
    },
    1325: {
        'kind': 'BoulderA',
        'xz': (20.672582170311216, -79.55828385036875),
        'original_y': 65.20034210344221,
        'corrected_y': 64.74313004876842,
    },
    1364: {
        'kind': 'BoulderA',
        'xz': (95.54688210000529, 368.98874664506764),
        'original_y': 5.288538399551772,
        'corrected_y': 5.163427675697716,
    },
    1541: {
        'kind': 'BoulderC',
        'xz': (-159.4744374993224, -63.083462964255204),
        'original_y': 85.85161233125542,
        'corrected_y': 85.708212317397,
    },
    1730: {
        'kind': 'BoulderC',
        'xz': (-57.30878549915775, -286.2323391796041),
        'original_y': 137.25853498797284,
        'corrected_y': 137.1313965994088,
    },
}


def corrected_height(index, kind, x, z, authored_y):
    """Keep every unrelated placement unchanged and reject recipe index drift."""
    correction = CORRECTIONS.get(index)
    if correction is None:
        return authored_y
    xx, zz = correction['xz']
    assert kind == correction['kind'] and abs(x - xx) < 1e-6 and abs(z - zz) < 1e-6, (
        'Reviewed island placement identity changed', index, kind, x, z)
    assert any(abs(authored_y - correction[key]) < 1e-4 for key in ('original_y', 'corrected_y')), (
        'Reviewed island placement elevation changed', index, authored_y)
    return correction['corrected_y']
