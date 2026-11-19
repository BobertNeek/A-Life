"""Mount the near vertical trim on existing ground only, never on the sea."""

from pathlib import Path
import runpy

H = Path(__file__).resolve().parent


def bind(ground, surf, base, near):
    return runpy.run_path(str(H.parent / "outfold/material.py"))["bind"](
        ground,
        surf,
        base,
        near,
        dict(
            meta="toe.json",
            colour="shelf-colour.png",
            roughness="shelf-roughness.png",
            height="shelf-height.png",
            depth=0.033,
            mix="OVERLAY",
            name="Wave-rubbed toeslab / split joints",
        ),
    )
