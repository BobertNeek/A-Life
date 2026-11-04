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

from care_frontier_sampler import integer, validate_bank, validate_layout

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
    if not isinstance(recipe, dict) or set(recipe) != {"schema", "legend", "scenarios"} or type(recipe["schema"]) is not int or recipe["schema"] != 1 or recipe["legend"] != LEGEND:
        raise ValueError("invalid recipe schema or legend")
    scenarios = recipe["scenarios"]
    if not isinstance(scenarios, list) or not 1 <= len(scenarios) <= 16:
        raise ValueError("bounded scenarios required")
    families = []
    for scenario in scenarios:
        if not isinstance(scenario, dict) or set(scenario) != {"id", "map", "condition", "difficulty", "goal", "random_roll"}:
            raise ValueError("invalid scenario fields")
        if not isinstance(scenario["id"], str):
            raise ValueError("scenario ID must be a string")
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


def recipe_schema(host=None):
    """Constrain decoding, not admission: native/host checks remain mandatory.

    The optional host preserves the request helper's original calling convention.
    The CLI always supplies it, restricting IDs and scenario count to its ledger.
    Cross-field rules (unique IDs, connected maps and contiguous difficulties)
    are still checked by compile_recipe, never repaired or silently rerolled.
    """
    ids = None
    if host is not None:
        if (not isinstance(host, dict) or set(host) != {
                "decision_budget", "slots", "sampler_seed", "current_difficulty",
                "pool_seeds", "holdout_seeds"}
                or not isinstance(host["pool_seeds"], dict)
                or not isinstance(host["holdout_seeds"], dict)):
            raise ValueError("invalid host settings")
        ids = list(host["pool_seeds"])
        if (not 1 <= len(ids) <= 16 or set(ids) != set(host["holdout_seeds"])
                or any(not isinstance(name, str) or not name.isascii()
                       or not name.replace("_", "").isalnum() or len(name) > 48 for name in ids)
                or not integer(host["current_difficulty"], 0, min(7, len(ids) - 1))):
            raise ValueError("invalid host family IDs or difficulty")

    def obj(properties):
        return {"type": "object", "properties": properties,
                "required": list(properties), "additionalProperties": False}

    def fixed(value):
        return {"type": "string", "const": value}

    family_id = {"type": "string", "minLength": 1, "maxLength": 48,
                 "pattern": "^[A-Za-z0-9_]+$"}
    if ids is not None:
        family_id["enum"] = ids
    scenario = obj({
        "id": family_id,
        "map": {"type": "array", "minItems": 3, "maxItems": 17,
                "items": {"type": "string", "minLength": 3, "maxLength": 17,
                          "pattern": "^#[#L.F]{1,15}#$"}},
        "condition": {"type": "string", "enum": ["newborn_v1", "ordinary_healthy_aging_v1"]},
        "difficulty": {"type": "integer", "minimum": 0,
                       "maximum": min(7, len(ids) - 1) if ids is not None else 7},
        "goal": fixed(GOAL),
        "random_roll": obj({
            "kind": fixed(RANDOM_ROLL["kind"]),
            "formula": fixed(RANDOM_ROLL["formula"]),
            "outcomes": {"type": "array", "const": list(RANDOM_ROLL["outcomes"]),
                         "minItems": 3, "maxItems": 3,
                         "items": {"type": "string", "enum": list(RANDOM_ROLL["outcomes"])}}}),
    })
    return obj({
        "schema": {"type": "integer", "const": 1},
        "legend": obj({key: fixed(value) for key, value in LEGEND.items()}),
        "scenarios": {"type": "array", "items": scenario,
                      "minItems": len(ids) if ids is not None else 1,
                      "maxItems": len(ids) if ids is not None else 16},
    })


class RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def request_recipe(endpoint, model, prompt, api_key=None, *, host=None, max_tokens=None,
                   no_think=False, response_output=None):
    # Never forward credentials across redirects or send them over plaintext.
    url = urlsplit(endpoint)
    if (url.scheme not in {"http", "https"} or not url.hostname or url.username or url.password
            or url.fragment or (url.scheme == "http" and url.hostname not in {"127.0.0.1", "::1", "localhost"})):
        raise ValueError("endpoint must be HTTPS or a loopback HTTP URL without userinfo")
    if max_tokens is not None and not integer(max_tokens, 1, 8192):
        raise ValueError("max_tokens must be an integer in 1..8192")
    if type(no_think) is not bool:
        raise ValueError("no_think must be a boolean")
    response_path = Path(response_output) if response_output is not None else None
    # Refuse known collisions before dispatch; exclusive creation below also
    # closes the race if another process creates the path during the request.
    if response_path is not None and os.path.lexists(response_path):
        raise FileExistsError("response output already exists: " + str(response_path))
    if no_think:
        prompt += " /no_think"
    # Explicitly configured endpoint; credentials come only from environment.
    payload = {"model": model, "messages": [{"role": "system", "content": prompt}],
               "response_format": {"type": "json_schema", "json_schema": {
                   "name": "alife_care_recipe", "strict": True, "schema": recipe_schema(host)}},
               "temperature": 0}
    if max_tokens is not None:
        payload["max_tokens"] = max_tokens
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = "Bearer " + api_key
    request = urllib.request.Request(endpoint, data=json.dumps(payload).encode(), headers=headers, method="POST")
    with urllib.request.build_opener(RejectRedirects()).open(request, timeout=60) as response:
        raw = response.read(1_000_001)
    if len(raw) > 1_000_000:
        raise ValueError("model response too large")
    if response_path is not None:
        # Only the bounded response body, byte for byte: no request headers,
        # credentials or synthetic envelope. Preserve malformed output as evidence.
        with response_path.open("xb") as out:
            out.write(raw)
    result = strict_json(raw)
    return strict_json(result["choices"][0]["message"]["content"])


def preflight_outputs(output, response_output=None):
    """Refuse output collisions before spending a model request."""
    output = Path(output)
    recipe_path = output.with_name(output.name + ".recipe.json")
    for path in (output, recipe_path):
        if os.path.lexists(path):
            raise FileExistsError("output already exists: " + str(path))
    if response_output is not None:
        response_path = Path(response_output)
        if response_path.resolve() in {output.resolve(), recipe_path.resolve()}:
            raise ValueError("response output must differ from bank and recipe outputs")
        if os.path.lexists(response_path):
            raise FileExistsError("response output already exists: " + str(response_path))
    return recipe_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--endpoint", required=True, help="explicit OpenAI-compatible chat completions URL")
    parser.add_argument("--model", required=True)
    parser.add_argument("--output", type=Path, required=True, help="new sampler-bank file")
    parser.add_argument("--max-tokens", type=int, help="optional generation cap in 1..8192 tokens")
    parser.add_argument("--no-think", action="store_true", help="append Qwen /no_think instruction (opt-in)")
    parser.add_argument("--response-output", type=Path, help="new private file for the bounded raw provider response")
    args = parser.parse_args()
    recipe_path = preflight_outputs(args.output, args.response_output)
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
    recipe = request_recipe(args.endpoint, args.model, prompt, os.environ.get("ALIFE_SCENARIO_API_KEY"),
                            host=host, max_tokens=args.max_tokens, no_think=args.no_think,
                            response_output=args.response_output)
    bank = compile_recipe(recipe, host)
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
