#!/usr/bin/env python3
"""Generate bounded care-sampler banks from an untrusted LLM recipe.

Maps encode bounded native wall and food layouts for feeding scenarios.
Unsupported symbols and unreachable layouts reject; native movement remains authoritative.
"""
import argparse
import json
import os
import urllib.request
from urllib.parse import urlsplit
from pathlib import Path

from care_frontier_sampler import validate_bank, validate_layout

# A finite 2 m grid; columns increase world X and rows world Z. The learner
# anchors the inherited founder. Native terrain owns wall and movement physics.
LEGEND = {"L": "learner", "F": "food", ".": "open", "#": "wall"}
GOAL = "nourish_and_survive"
RANDOM_ROLL = {"kind": "food_variety_by_seed_v1", "formula": "(seed // 12) % 3",
               "outcomes": ["root", "fruit", "seed"]}


def map_layout(rows):
    layout = {"rows": rows}
    validate_layout(layout)
    # Fast syntactic connectivity precheck. The native terrain authority still
    # validates body clearance and movement over every path edge.
    start = next((x, z) for z, row in enumerate(rows) for x, c in enumerate(row) if c == "L")
    target = next((x, z) for z, row in enumerate(rows) for x, c in enumerate(row) if c == "F")
    seen, todo = {start}, [start]
    for x, z in todo:
        for nx, nz in ((x+1,z), (x-1,z), (x,z+1), (x,z-1)):
            if (0 <= nz < len(rows) and 0 <= nx < len(rows[0]) and rows[nz][nx] != "#"
                    and (nx,nz) not in seen):
                seen.add((nx,nz)); todo.append((nx,nz))
    if target not in seen:
        raise ValueError("food is unreachable in the proposed grid")
    return layout


def compile_recipe(recipe, host):
    if not isinstance(recipe, dict) or set(recipe) != {"schema", "legend", "scenarios"} or recipe["schema"] != 1 or recipe["legend"] != LEGEND:
        raise ValueError("invalid recipe schema or legend")
    scenarios = recipe["scenarios"]
    if not isinstance(scenarios, list) or not 1 <= len(scenarios) <= 16:
        raise ValueError("bounded scenarios required")
    families = []
    for scenario in scenarios:
        if not isinstance(scenario, dict) or set(scenario) != {"id", "map", "condition", "difficulty", "goal", "random_roll"}:
            raise ValueError("invalid scenario fields")
        layout = map_layout(scenario["map"])
        if scenario["goal"] != GOAL or scenario["random_roll"] != RANDOM_ROLL:
            raise ValueError("unsupported goal or random roll")
        families.append({"id": scenario["id"], "lesson": "feeding",
                         "food_need_condition": scenario["condition"],
                         "placement": "ascii_layout_v1", "layout": layout,
                         "difficulty": scenario["difficulty"]})
    if not isinstance(host, dict) or set(host) != {"decision_budget", "slots", "sampler_seed", "current_difficulty", "pool_seeds", "holdout_seeds"}:
        raise ValueError("invalid host settings")
    # Seeds and budgets are host supplied, never model supplied. A seed per
    # family is allocated in deterministic order from each disjoint ledger.
    pool, holdouts = [], []
    for name, target in (("pool_seeds", pool), ("holdout_seeds", holdouts)):
        ledger = host[name]
        if not isinstance(ledger, dict) or set(ledger) != {f["id"] for f in families}:
            raise ValueError("seed ledger must cover exactly the generated family IDs")
        for family in families:
            values = ledger[family["id"]]
            if not isinstance(values, list) or not values:
                raise ValueError("seed ledger must contain nonempty lists")
            target.extend({"family": family["id"], "seed": s} for s in values)
    bank = {"schema": 1, **{k: host[k] for k in ("decision_budget", "slots", "sampler_seed", "current_difficulty")},
            "families": families, "pool": pool, "holdouts": holdouts}
    validate_bank(bank)
    return bank


def strict_json(raw):
    def unique(pairs):
        obj = {}
        for key, value in pairs:
            if key in obj:
                raise ValueError("duplicate JSON key")
            obj[key] = value
        return obj
    return json.loads(raw, object_pairs_hook=unique, parse_constant=lambda _: (_ for _ in ()).throw(ValueError("non-finite JSON number")))


class RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def request_recipe(endpoint, model, prompt, api_key=None):
    # Never forward credentials across redirects or send them over plaintext.
    url = urlsplit(endpoint)
    if (url.scheme not in {"http", "https"} or not url.hostname or url.username or url.password
            or url.fragment or (url.scheme == "http" and url.hostname not in {"127.0.0.1", "::1", "localhost"})):
        raise ValueError("endpoint must be HTTPS or a loopback HTTP URL without userinfo")
    # Explicitly configured endpoint; credentials come only from environment.
    payload = {"model": model, "messages": [{"role": "system", "content": prompt}],
               "response_format": {"type": "json_object"}, "temperature": 0}
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = "Bearer " + api_key
    request = urllib.request.Request(endpoint, data=json.dumps(payload).encode(), headers=headers, method="POST")
    with urllib.request.build_opener(RejectRedirects()).open(request, timeout=60) as response:
        raw = response.read(1_000_001)
    if len(raw) > 1_000_000:
        raise ValueError("model response too large")
    result = strict_json(raw)
    return strict_json(result["choices"][0]["message"]["content"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--endpoint", required=True, help="explicit OpenAI-compatible chat completions URL")
    parser.add_argument("--model", required=True)
    parser.add_argument("--output", type=Path, required=True, help="new sampler-bank file")
    args = parser.parse_args()
    host = strict_json(args.host.read_text(encoding="utf-8"))
    prompt = ("Return JSON only with schema=1, legend=" + json.dumps(LEGEND) +
              ", and scenarios array. Each scenario has id, map (rectangular array of strings), condition, difficulty, "
              "goal='nourish_and_survive', and random_roll=" + json.dumps(RANDOM_ROLL) + ". The map is 3..17 rows/columns, "
              "has exactly one L and one F, uses '#' for walls and '.' for open space, with a complete wall border. "
              "Learner and food must connect by cardinal open cells. Example: ['#####','#L.F#','#####']. "
              "Each cell is 2 world units, columns increase X, rows increase Z. "
              "Allowed conditions: newborn_v1, ordinary_healthy_aging_v1. "
              "Use exactly these family IDs: " + json.dumps(list(host["pool_seeds"])) +
              ". Use distinct maps/condition pairs. Difficulties form contiguous integers starting at 0 and include "
              + str(host["current_difficulty"]) + ". Do not add fields.")
    recipe = request_recipe(args.endpoint, args.model, prompt, os.environ.get("ALIFE_SCENARIO_API_KEY"))
    bank = compile_recipe(recipe, host)
    recipe_path = args.output.with_name(args.output.name + ".recipe.json")
    # Preserve the untrusted proposal alongside its compiled artifact. Never
    # overwrite an earlier run, including a partial run.
    with recipe_path.open("x", encoding="utf-8") as out:
        json.dump({"model": args.model, "recipe": recipe}, out, sort_keys=True, allow_nan=False)
        out.write("\n")
    with args.output.open("x", encoding="utf-8") as out:
        json.dump(bank, out, sort_keys=True, allow_nan=False)
        out.write("\n")


if __name__ == "__main__":
    main()
