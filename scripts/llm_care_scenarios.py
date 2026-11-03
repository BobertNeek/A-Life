#!/usr/bin/env python3
"""Generate bounded care-sampler banks from an untrusted LLM recipe.

Maps encode validated relative food placement in native feeding scenarios.
Unsupported symbols or placements reject; native world construction remains authoritative.
"""
import argparse
import json
import os
import urllib.request
from pathlib import Path

from care_frontier_sampler import validate_bank

# Grid coordinates are relative to the creature. One cell is one world unit;
# columns increase world X, rows increase world Z. Only feeding scenes are
# supported. Other entities remain under native scene construction.
LEGEND = {"C": "creature", "F": "food", ".": "unspecified"}
GOAL = "nourish_and_survive"


def map_offset(rows):
    if (not isinstance(rows, list) or not 1 <= len(rows) <= 9
            or any(not isinstance(row, str) or not 1 <= len(row) <= 9 for row in rows)
            or len({len(row) for row in rows}) != 1):
        raise ValueError("rectangular 1..9-cell ASCII map required")
    cells = [(x, z, ch) for z, row in enumerate(rows) for x, ch in enumerate(row)]
    if any(ch not in LEGEND for _, _, ch in cells):
        raise ValueError("unsupported map symbol")
    creature = [(x, z) for x, z, ch in cells if ch == "C"]
    food = [(x, z) for x, z, ch in cells if ch == "F"]
    if len(creature) != 1 or len(food) != 1:
        raise ValueError("exactly one creature and one food required")
    offset = [food[0][0] - creature[0][0], food[0][1] - creature[0][1]]
    if not 4 <= sum(n*n for n in offset) <= 16:
        raise ValueError("food must be 2 to 4 world units from creature")
    return offset


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
        offset = map_offset(scenario["map"])
        if scenario["goal"] != GOAL or scenario["random_roll"] != "native_seeded_v1":
            raise ValueError("unsupported goal or random roll")
        families.append({"id": scenario["id"], "lesson": "feeding",
                         "food_need_condition": scenario["condition"],
                         "placement": "ascii_grid_v1", "food_offset": offset,
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


def request_recipe(endpoint, model, prompt, api_key=None):
    # Explicitly configured endpoint; credentials come only from environment.
    payload = {"model": model, "messages": [{"role": "system", "content": prompt}],
               "response_format": {"type": "json_object"}, "temperature": 0}
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = "Bearer " + api_key
    request = urllib.request.Request(endpoint, data=json.dumps(payload).encode(), headers=headers, method="POST")
    with urllib.request.urlopen(request, timeout=60) as response:
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
              "goal='nourish_and_survive', random_roll='native_seeded_v1'. The map has exactly one C and F; "
              "all other cells are '.', meaning unspecified terrain, not cleared space. One cell is one world unit, columns are X and rows are Z. "
              "Food must be 2–4 units from the creature, for example ['C..F']. "
              "Allowed conditions: newborn_v1, ordinary_healthy_aging_v1. "
              "Use exactly these family IDs: " + json.dumps(list(host["pool_seeds"])) +
              ". Use distinct map/condition pairs. Difficulties form contiguous integers starting at 0 and include "
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
