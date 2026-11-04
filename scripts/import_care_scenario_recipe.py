#!/usr/bin/env python3
"""Validate an external JSON care recipe and create a new local sampler bank.

This offline bridge makes no provider or authorship claim. It reads only the
explicit recipe and host files; native movement validation is still required.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import sys

from llm_care_scenarios import compile_recipe, strict_json

MAX_INPUT_BYTES = 1_000_000


def reject_symlinks(path):
    # Refuse both leaf links and directory aliases, including dangling links.
    for part in (path.absolute(), *path.absolute().parents):
        if part.is_symlink():
            raise ValueError('symlink paths are not supported')


def read_json(path):
    reject_symlinks(path)
    # Nonblocking + fstat avoids waiting on a FIFO/device disguised as JSON.
    flags = os.O_RDONLY | getattr(os, 'O_NONBLOCK', 0) | getattr(os, 'O_NOFOLLOW', 0)
    descriptor = os.open(path, flags)
    with os.fdopen(descriptor, 'rb') as source:
        if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
            raise ValueError('input must be a regular file')
        raw = source.read(MAX_INPUT_BYTES + 1)
    if len(raw) > MAX_INPUT_BYTES:
        raise ValueError('input exceeds the 1000000-byte limit')
    value = strict_json(raw)

    def finite(item):
        if isinstance(item, float) and not math.isfinite(item):
            raise ValueError('non-finite JSON number')
        if isinstance(item, dict):
            for child in item.values():
                finite(child)
        elif isinstance(item, list):
            for child in item:
                finite(child)
    finite(value)
    return value, hashlib.sha256(raw).hexdigest()


def import_recipe(recipe_path, host_path, output_path):
    recipe_path, host_path, output_path = map(Path, (recipe_path, host_path, output_path))
    for path in (recipe_path, host_path, output_path):
        reject_symlinks(path)
    resolved = [p.resolve() for p in (recipe_path, host_path, output_path)]
    if len(set(resolved)) != 3:
        raise ValueError('recipe, host and output must be distinct paths')
    if os.path.lexists(output_path):
        raise FileExistsError('output already exists')
    if os.path.samefile(recipe_path, host_path):
        raise ValueError('recipe and host must be distinct files')
    recipe, recipe_hash = read_json(recipe_path)
    host, host_hash = read_json(host_path)
    # Older compile_recipe revisions accept True == 1; never admit that here.
    if not isinstance(recipe, dict) or type(recipe.get('schema')) is not int:
        raise ValueError('recipe schema must be an integer')
    bank = compile_recipe(recipe, host)
    raw = (json.dumps(bank, sort_keys=True, allow_nan=False) + '\n').encode('utf-8')
    # Validate and serialize before any output creation. Exclusive creation also
    # closes a final-path collision race. Never overwrite or repair a partial run.
    with output_path.open('xb') as output:
        output.write(raw)
    return {'source': 'external_json', 'recipe_sha256': recipe_hash,
            'host_sha256': host_hash, 'bank_sha256': hashlib.sha256(raw).hexdigest(),
            'families': len(bank['families'])}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--recipe', type=Path, required=True)
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        receipt = import_recipe(args.recipe, args.host, args.output)
    except (OSError, ValueError, TypeError, KeyError, RecursionError):
        # Do not echo untrusted contents, paths or host settings in error output.
        print('Recipe import rejected: check JSON, host validation, file size and distinct new output paths.',
              file=sys.stderr)
        return 1
    print(json.dumps(receipt, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
