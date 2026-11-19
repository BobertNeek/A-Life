"""Shared aligned stream authoring for the glacial rocks and true-slope deposit."""

import numpy as np


def streams(out, buf):
    def write(
        stream, component_type, kind, *, indices=False, bounds=False, normalized=False
    ):
        stream = np.ascontiguousarray(stream)
        buf.extend(b"\0" * (-len(buf) % 4))
        off = len(buf)
        buf.extend(stream.tobytes())
        vi = len(out["bufferViews"])
        out["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=off,
                byteLength=stream.nbytes,
                target=34963 if indices else 34962,
            )
        )
        accessor = dict(
            bufferView=vi, componentType=component_type, count=len(stream), type=kind
        )
        if bounds:
            accessor.update(min=stream.min(0).tolist(), max=stream.max(0).tolist())
        if normalized:
            accessor["normalized"] = True
        ix = len(out["accessors"])
        out["accessors"].append(accessor)
        return ix

    return write
