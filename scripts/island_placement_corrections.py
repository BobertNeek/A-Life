"""Reviewed, instance-specific island authoring corrections (metres, Y-up).

Do not ground the rock library globally: its authored elevations are intentional.
The sampled near terrain differs from the analytic authoring function here.
Each target has independent triangle-contact and exposed-gap evidence. Heights
embed the relevant rock LODs by 2 cm plus their reviewed support-height spread.
BoulderA 1325 supports visible L0/L1 only: its approved L2 simplification would
require overburying those meshes. Current production culling never displays
non-tree L2; the actual cull/LOD policy is locked by a renderer regression.
"""

CORRECTIONS = {
    1319: {
        'kind': 'BoulderC',
        'proxy_index': 1310,
        'xz': (35.826016966114025, -212.13306068550276),
        'original_y': 155.7075371101759,
        'corrected_y': 154.78496851454122,
        'support_lods': (0, 1, 2),
        'original_clearance_m': (0.8751421120024361, 0.9025724844045158, 0.8785104412512794),
        'maximum_embed_m': 0.051,
    },
    1325: {
        'kind': 'BoulderA',
        'proxy_index': 1316,
        'xz': (20.672582170311216, -79.55828385036875),
        'original_y': 65.20034210344221,
        'corrected_y': 64.74313004876842,
        'support_lods': (0, 1),
        'original_clearance_m': (0.4372102222275913, 0.37381539542032627, 0.8256465596722276),
        'maximum_embed_m': 0.085,
    },
    1364: {
        'kind': 'BoulderA',
        'proxy_index': 1348,
        'xz': (95.54688210000529, 368.98874664506764),
        'original_y': 5.288538399551772,
        'corrected_y': 5.163427675697716,
        'support_lods': (0, 1, 2),
        'original_clearance_m': (0.08964150304593466, 0.09317277368130839, 0.10511078026541892),
        'maximum_embed_m': 0.037,
    },
    1541: {
        'kind': 'BoulderC',
        'proxy_index': 1489,
        'xz': (-159.4744374993224, -63.083462964255204),
        'original_y': 85.85161233125542,
        'corrected_y': 85.708212317397,
        'support_lods': (0, 1, 2),
        'original_clearance_m': (0.1155341256958593, 0.12236049425697204, 0.12340358836471754),
        'maximum_embed_m': 0.03,
    },
    1730: {
        'kind': 'BoulderC',
        'proxy_index': 1631,
        'xz': (-57.30878549915775, -286.2323391796041),
        'original_y': 137.25853498797284,
        'corrected_y': 137.1313965994088,
        'support_lods': (0, 1, 2),
        'original_clearance_m': (0.10419544422979499, 0.10365330631189806, 0.10713306367716768),
        'maximum_embed_m': 0.025,
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
