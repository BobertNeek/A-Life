"""Two cameras on one registered dry hollow; never positioned again between sides."""

TAGS = ("slack-guardian", "slack-eye")


def frame(tag, h):
    if tag == "slack-guardian":
        return (tag, (410.0, 72.0, 36.0), (334.0, 4.4, -49.0), 132.0, 39, 1)
    if tag == "slack-eye":
        x, z, tx, tz = 329.6, -9.8, 327.2, -19.9
        return (tag, (x, h(x, z) + 1.04, z), (tx, h(tx, tz) + 0.53, tz), 0, 27, 0)
    raise ValueError(tag)
