"""Pure finalization seam proposal. No dispatch, native execution or file reads.

Call after constructing existing finalize() outputs and BEFORE SHA256SUMS.
Retain full-case ledger validation/complete_pairs unchanged. Prefix admission is
a separate ledger map, and these results never populate ledger.validated.
"""
import copy
import json
import report_contract as c
from scalar_prefix_contract import ORDER

PUBLIC_CAP = 16 * 1024**2


def _hashes(outputs):
    result = dict(outputs)
    result.pop('SHA256SUMS', None)
    result['SHA256SUMS'] = ''.join(f'{c.sha(raw)}  {name}\n'
        for name, raw in sorted(result.items())).encode()
    return result


def add_prefixes_and_bound(outputs, prefixes, public_cap=None):
    """Fixed all-rows-or-summary-only fallback, independent of observed behavior.

    Required summaries/receipts are NEVER dropped. Before dispatch, separately
    prove their worst-case bound plus exact source patches fits PUBLIC_CAP. If
    that invariant is broken, fail before writing rather than raise the limit.
    """
    cap = PUBLIC_CAP if public_cap is None else public_cap
    result = dict(outputs)
    result.pop('SHA256SUMS', None)
    summary = json.loads(result['summary.json'])
    rows = json.loads(result['rows.json'])
    checks = json.loads(result['checks.json'])
    full_ids = {x['case_id'] for x in summary['cases']}
    c.need(type(prefixes) is dict and set(prefixes) <= set(ORDER), 'case_id')
    c.need(not full_ids.intersection(prefixes), 'accounting')
    admitted = [prefixes[cid] for cid in ORDER if cid in prefixes]
    for cid in prefixes:
        p = prefixes[cid]
        c.need(p['summary']['case_id'] == cid and p['summary']['completed_case'] is False
               and p['summary']['comparison_eligible'] is False, 'accounting')
    summary['incomplete_prefixes'] = [copy.deepcopy(p['summary']) for p in admitted]
    rows['incomplete_prefixes'] = [dict(header=copy.deepcopy(p['header']),
        header_sha256=p['header_sha256'], rows=copy.deepcopy(p['rows'])) for p in admitted]
    checks['prefix_admission_does_not_change_case_sets'] = True
    checks['strict_prefix_cases'] = [cid for cid in ORDER if cid in prefixes]
    checks['strict_prefix_bindings'] = {cid: dict(header=copy.deepcopy(prefixes[cid]['header']),
        header_sha256=prefixes[cid]['header_sha256']) for cid in ORDER if cid in prefixes}
    checks['row_export'] = 'all_validated_case_and_prefix_rows'
    result.update({'summary.json': c.canonical(summary), 'rows.json': c.canonical(rows),
                   'checks.json': c.canonical(checks)})
    result = _hashes(result)
    if sum(map(len, result.values())) > cap:
        # This rule is frozen before observation: omit every detailed row, not
        # whichever arm/event is inconvenient. Keep all scalar summaries/counts.
        result['rows.json'] = c.canonical(dict(cases=[], incomplete_prefixes=[],
            omitted_validated_cases=[cid for cid in ORDER if cid in full_ids],
            omitted_prefix_cases=[cid for cid in ORDER if cid in prefixes],
            reason='public_byte_cap'))
        checks['row_export'] = 'summary_only_due_public_byte_cap'
        result['checks.json'] = c.canonical(checks)
        result['report.md'] += (b'\nDetailed scalar rows omitted under the frozen public byte cap; '
            b'all validated case summaries, completion receipts and strict incomplete '
            b'prefix summaries/counts are retained. Prefixes are not completed cases or pairs.\n')
        result = _hashes(result)
    c.need(sum(map(len, result.values())) <= cap, 'public_bound')
    return result
