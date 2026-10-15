"""Reviewed, instance-specific island authoring corrections (metres, Y-up).

Do not ground the rock library globally: its authored elevations are intentional.
The sampled near terrain differs from the analytic authoring function here.
Triangle/triangle support against all three shipping BoulderC LODs gives resting
origins 154.832398887, 154.804968515 and 154.829030558. A 2 cm embed below the
lowest origin keeps each LOD supported, with less than 5 cm penetration.
"""

PLACEMENT_INDEX = 1319
PROXY_INDEX = 1310
KIND = 'BoulderC'
XZ = (35.826016966114025, -212.13306068550276)
ORIGINAL_Y = 155.7075371101759
CORRECTED_Y = 154.78496851454123


def corrected_height(index, kind, x, z, authored_y):
    """Keep every unrelated placement unchanged and reject recipe index drift."""
    if index != PLACEMENT_INDEX:
        return authored_y
    assert kind == KIND and abs(x - XZ[0]) < 1e-6 and abs(z - XZ[1]) < 1e-6, (
        'Reviewed island placement identity changed', index, kind, x, z)
    assert abs(authored_y - ORIGINAL_Y) < 1e-4 or abs(authored_y - CORRECTED_Y) < 1e-4, (
        'Reviewed island placement elevation changed', authored_y)
    return CORRECTED_Y
