"""Select a stable phenotype for art only. The native renderer will use the same simple field if selected."""

import math


def family(index, x, y, z):
    rank = ((index % 17) * 7 % 17) / 16
    shelter = 0.5 * math.sin(0.041 * x + 0.028 * z) + 0.5 * math.cos(
        0.031 * z - 0.011 * x
    )
    if y < 4.35 and rank < 0.81 or shelter < -0.34 and rank < 0.56:
        return "GrassSilver"
    if rank > 0.86 or shelter > 0.25 and rank > 0.63:
        return "GrassAmber"
    return "Grass"
