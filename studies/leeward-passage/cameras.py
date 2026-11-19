"""Locked physical camera stations, shared by both sides; Blender CPU art."""


def frame(name, ground):
    if name == "reach-guardian":
        return (name, (306.0, 142.0, 431.0), (104.0, 5.1, 367.0), 231.0, 35, 1)
    if name == "reach-meadow":
        x, z, tx, tz = 76.60, 315.50, 73.50, 300.15
        return (
            name,
            (x, ground(x, z) + 1.04, z),
            (tx, ground(tx, tz) + 0.37, tz),
            0.0,
            30,
            0,
        )
    if name == "reach-alder":
        # At the traversable lower turn, looking back up the bank so an
        # entire *real* 7 m alder relates the new root and soil to animal scale.
        x, z, tx, tz = 128.90, 453.40, 132.49, 444.79
        return (
            name,
            (x, ground(x, z) + 1.04, z),
            (tx, ground(tx, tz) + 1.72, tz),
            0.0,
            23,
            0,
        )
    if name == "reach-strand":
        x, z, tx, tz = 126.82, 446.35, 132.4, 460.56
        return (
            name,
            (x, ground(x, z) + 1.04, z),
            (tx, ground(tx, tz) + 0.42, tz),
            0.0,
            28,
            0,
        )
    raise ValueError(name)
