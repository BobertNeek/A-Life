"""Source-only, pure scalar-prefix admission. No native launch or checkpoint I/O.

`report_contract.validate_rows` is the unmodified study row loop factored out of
validate_case. A prefix never invokes or substitutes for completed-case admission.
"""
import copy
import io
import report_contract as c
import diagnostic_contract as diagnostic

CASE_CAP = 8 * 1024**2
RECORD_CAP = 64 * 1024
CASES = {f's{i+1}_{w}': (2026112301+i, w)
         for i in range(2) for w in ('old_world', 'new_world')}
ORDER = ('s1_old_world', 's1_new_world', 's2_new_world', 's2_old_world')
HEADER = c.obj(schema_version=c.enum(1), scope=c.enum('study_scalar_prefix'),
    case_id=c.enum(*CASES), seed=c.enum(2026112301, 2026112302),
    world=c.enum('old_world', 'new_world'), initial=c.PAIR, source=c.SOURCE)


def _record(stream, used):
    """Read one bounded JSONL record; one discarded sentinel detects overflow."""
    remaining = CASE_CAP - used
    limit = min(RECORD_CAP, remaining)
    try:
        raw = stream.readline(limit + 1)
    except OSError:
        return None, used, 'scalar_read_error'
    c.need(type(raw) is bytes, 'schema')
    if not raw:
        return None, used, 'eof'
    if len(raw) > limit:
        return None, used, 'case_byte_cap' if remaining <= RECORD_CAP else 'record_byte_cap'
    if not raw.endswith(b'\n'):
        return None, used, 'truncated_record'
    return raw, used + len(raw), None


def _summary(header, rows, terminal, end, rejected_index):
    # No native counters enter this reduction. Everything is derived from rows.
    ds = [d for row in rows for d in row['decisions']]
    observed = dict(primary_ticks=len(rows), decisions=len(ds),
        harmful_meals=sum(d['physical_meals'][0] for d in ds),
        safe_meals=sum(d['physical_meals'][1] for d in ds),
        manipulation_channels=sum(ch['slot'] == 2 for d in ds for ch in d['channels']),
        manipulation_transfers=sum(len(ch['transfers']) for d in ds for ch in d['channels']),
        physical_episode_coverage_active=[sum(d['ingest_episode_active'][i] for d in ds) for i in range(2)],
        physical_episode_coverage_staged=[sum(d['ingest_episode_staged'][i] for d in ds) for i in range(2)],
        exact_ingest_recall_decisions=sum(any(z['source_exact_ingest'] for z in d['candidates']) for d in ds),
        sleeping_ticks=sum(row['sleeping'] for row in rows),
        sleep_consolidations=sum(row['sleep_consolidations'] for row in rows))
    # Reuse reviewed metrics for the less elementary observational counters, but
    # do not export its complete-study inference or rate fields for a prefix.
    if rows:
        m = c.metrics({'rows': rows})
        for key in ('h_held_s_eat_attempts', 'h_to_s_atomic_swaps', 'h_held_s_eat_failures',
                    's_contact_while_h_held_decisions', 'positive_confidence_ingest_recall_decisions'):
            observed[key] = m[key]
    else:
        observed.update(h_held_s_eat_attempts=0, h_to_s_atomic_swaps=0,
            h_held_s_eat_failures=0, s_contact_while_h_held_decisions=0,
            positive_confidence_ingest_recall_decisions=0)
    return dict(schema_version=1, case_id=header['case_id'], seed=header['seed'],
        world=header['world'], status='PARTIAL', prefix_validation='strictly_validated',
        completed_case=False, comparison_eligible=False,
        evidence_scope='contiguous_completed_scalar_rows_only_lower_bound',
        admitted_rows=len(rows), terminal_row_observed=terminal,
        stream_end=end, first_unadmitted_row=rejected_index,
        admitted_rows_sha256=c.sha(c.canonical(rows)), observed=observed,
        physiology=(dict(min_health=min(r['health'] for r in rows),
            max_injury=max(r['injury'] for r in rows), last_energy=rows[-1]['energy'],
            last_alive=rows[-1]['alive']) if rows else None),
        preservation='completed_case_founder_and_roundtrip_checks_not_established')


def admit_stream(stream, expected_case_id, expected_initial, binding, *, start_verified=False):
    """Admit the longest strict prefix; never skip a bad row or inspect its tail.

    `expected_initial` must come from the independently admitted PREPARED marker
    and `binding` from the frozen source/executable proof. A caller must also have
    a verified START request/started marker for this case. JSONL contains only a
    closed header and closed ROW objects, not journals, errors or checkpoints.
    """
    c.need(expected_case_id in CASES, 'case_id')
    c.validate(expected_initial, c.PAIR)
    raw, used, end = _record(stream, 0)
    c.need(raw is not None, 'prefix_header')
    header = c.parse(raw, HEADER)
    seed, world = CASES[expected_case_id]
    c.need((header['case_id'], header['seed'], header['world']) ==
           (expected_case_id, seed, world), 'case_id')
    c.need(header['initial'] == expected_initial and
           expected_initial['asset_sha256'] == c.ASSET and
           expected_initial['sampling_seed'] == seed, 'initial_binding')
    c.need(type(binding) is dict and set(binding) == {'old_world', 'new_world'}, 'source_binding')
    for source in binding.values():
        c.validate(source, c.SOURCE)
    c.need(header['source'] == binding[world], 'source_binding')
    rows, terminal, rejected_index = [], None, None
    while True:
        raw, used, end = _record(stream, used)
        if raw is None:
            if end != 'eof':
                rejected_index = len(rows) + 1
            break
        c.need(start_verified is True, 'start_binding')
        try:
            row = c.parse(raw, c.ROW)
            candidate = rows + [row]
            # Transactional: the previous list/state is unchanged on rejection.
            candidate_terminal = c.validate_rows(candidate)
        except c.Reject:
            # Finite public reason; neither exception text nor rejected bytes.
            end, rejected_index = 'invalid_scalar_row', len(rows) + 1
            break
        rows.append(row)
        terminal = candidate_terminal
    return dict(header=copy.deepcopy(header), header_sha256=c.sha(c.canonical(header)),
        summary=_summary(header, rows, terminal, end, rejected_index), rows=rows)


def admit_bytes(raw, expected_case_id, expected_initial, binding, *, start_verified=False):
    c.need(type(raw) is bytes, 'schema')
    return admit_stream(io.BytesIO(raw), expected_case_id, expected_initial, binding,
                        start_verified=start_verified)


def assess_progress(raw, prefix):
    """Independent, optional final durable progress. Invalid progress loses no rows.

    `raw` is the existing finite rejection receipt from the same bound native
    process. Caller guarantees quiescence and a verified source/initial/START
    context. Hard crashes without a final receipt pass None for horizon bounds.
    Returned but unprojected steps supply NO physical or behavioral evidence.
    """
    n = prefix['summary']['admitted_rows']
    unknown = dict(status='unavailable_or_invalid', completed_host_steps_minimum=n,
        completed_host_steps_maximum=256, unadmitted_requested_host_steps=None,
        requested_not_returned=None, returned_unprojected=None,
        projected_uncollected=None, collected_not_admitted=None,
        behavioral_evidence_rows=n)
    if raw is None:
        return unknown
    try:
        c.need(type(raw) is bytes and len(raw) <= 16 * 1024, 'output_bound')
        x = diagnostic.parse_rejection(raw, prefix['header']['case_id'])
        p = x['progress']
        c.need(n <= p['collected_rows'], 'diagnostic_counts')
        c.need(p['host_returned'] <= p['collected_rows'] + 1, 'diagnostic_counts')
        c.need(p['host_requested'] <= p['collected_rows'] + 1, 'diagnostic_counts')
        c.need(prefix['summary']['observed']['decisions'] <= p['replay_returned'],
               'diagnostic_counts')
        c.need(p['preparation_requested'] == p['preparation_returned'] == 600,
               'diagnostic_counts')
    except c.Reject:
        return unknown
    return dict(status='strictly_validated_progress_separate_from_row_evidence',
        completed_host_steps_minimum=p['host_returned'],
        completed_host_steps_maximum=p['host_requested'],
        unadmitted_requested_host_steps=p['host_requested'] - n,
        requested_not_returned=p['host_requested'] - p['host_returned'],
        returned_unprojected=p['host_returned'] - p['projected_steps'],
        projected_uncollected=p['projected_steps'] - p['collected_rows'],
        collected_not_admitted=p['collected_rows'] - n,
        behavioral_evidence_rows=n)
