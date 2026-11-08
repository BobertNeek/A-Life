"""Source-only motor-lobe construction spec. NOT an admitted A-Life runtime.

No world, checkpoint, asset, optimizer, sampler, native build, or network access.
Produces a separate manifest. New typed constant-reference afferents and appended
arbitration readout head are required; these are NOT current class-5 abilities.
"""
from dataclasses import dataclass, asdict
from math import tanh
import json

SELECTOR = 'ProspectiveMatchedMotorArbitrationV1'  # No assigned capacity ID.
BASE_NEURONS = 2048
DECLARED_NEURONS = 2112  # 64 appended, 52 active, 12 explicitly reserved.
DRIVES = ('hunger', 'fatigue', 'fear', 'pain', 'loneliness', 'curiosity',
          'ATP_availability', 'unsigned_temperature_stress', 'reproductive_readiness',
          'bound_boredom', 'unbound_extension1')
SETPOINTS = (.35, .35, .25, .10, .35, .35, .50, .10, .50, .35, .50)
FAMILIES = ('Idle', 'Rest', 'Inspect', 'Approach', 'Avoid', 'Contact', 'Ingest', 'Other')
OBJECT, POS_CHEM, NEG_CHEM, HOT, COLD, PEER, VOCAL, THERMAL, NOVELTY, PLAY = range(36, 46)

@dataclass(frozen=True)
class Pair:
    name: str
    signal: int
    reference: int

@dataclass(frozen=True)
class Branch:
    target: int
    inputs: tuple  # Exact (neuron address, signed coefficient) list.
    threshold: float = 0.0
    output_gain: float = 1.0

@dataclass(frozen=True)
class Readout:
    family: str
    source: int
    lane: int
    weight: float

AFFERENTS = [Pair('afferent_' + name, 2048 + 2*i, 2049 + 2*i)
             for i, name in enumerate(DRIVES)]
HIGH = [Pair(name + '_above_setpoint', 2070 + 2*i, 2071 + 2*i)
        for i, name in enumerate(DRIVES)]
ATP_LOW = Pair('ATP_below_setpoint', 2092, 2093)
REST = Pair('recovery_motor_state', 2094, 2095)
SEEK = Pair('voluntary_search_motor_state', 2096, 2097)
ESCAPE = Pair('fear_motor_state', 2098, 2099)
ALL_PAIRS = AFFERENTS + HIGH + [ATP_LOW, REST, SEEK, ESCAPE]

def expand(*terms):
    """Signed differential branch inputs, never coefficient * raw activation."""
    return tuple(entry for pair, weight in terms
                 for entry in ((pair.signal, weight), (pair.reference, -weight)))

def branches():
    result = []
    # Each reference gets the same branch slots, with zero weights. Its branch
    # output is exactly zero, not a fabricated tonic activity or tanh(-theta).
    for src, dst in zip(AFFERENTS, HIGH):
        result += [Branch(dst.signal, expand((src, 4.0))),
                   Branch(dst.reference, expand((src, 0.0)))]
    result += [Branch(ATP_LOW.signal, expand((AFFERENTS[6], -4.0))),
               Branch(ATP_LOW.reference, expand((AFFERENTS[6], 0.0)))]
    states = [
        (REST, [(HIGH[1], 1.25), (ATP_LOW, 1.0), (HIGH[3], .15),
                (HIGH[0], -.75), (ESCAPE, -1.5), (SEEK, -.5)]),
        (SEEK, [(HIGH[0], .75), (HIGH[5], .30), (HIGH[9], .20),
                (REST, -1.25), (ATP_LOW, -.50), (ESCAPE, -.75)]),
        # No chronic-pain or bitter-is-poison oracle here. Acute localized pain
        # keeps its independent physical withdrawal path. Fear is broad alarm.
        (ESCAPE, [(HIGH[2], 1.25), (REST, -.05)]),
    ]
    for dst, terms in states:
        result.append(Branch(dst.signal, expand(*terms)))
        result.append(Branch(dst.reference, expand(*[(p, 0.0) for p, _ in terms])))
    return result

def readouts():
    result = []
    def add(family, pair, lane, coefficient):
        result.extend([Readout(family, pair.signal, lane, coefficient),
                       Readout(family, pair.reference, lane, -coefficient)])
    # Rest raw23 is an intrinsic constant in the Rest family only.
    for pair, weight in [(REST, 4.0), (SEEK, -1.5), (ESCAPE, -4.0)]:
        add('Rest', pair, 23, weight)
    add('Rest', HIGH[7], THERMAL, -2.0)
    for pair, weight in [(REST, -4.0), (SEEK, 1.25), (ESCAPE, -2.0)]:
        add('Approach', pair, OBJECT, weight)
    for lane, weight in [(POS_CHEM, .5), (NEG_CHEM, -.5)]:
        add('Approach', HIGH[0], lane, weight)
    for lane in [HOT, COLD]:
        add('Approach', HIGH[7], lane, -2.0)
        add('Avoid', HIGH[7], lane, 3.0)
    add('Approach', HIGH[4], PEER, .5)
    add('Approach', HIGH[8], PEER, .25)
    add('Avoid', REST, OBJECT, -2.0)
    add('Avoid', ESCAPE, OBJECT, 5.0)
    add('Avoid', ESCAPE, NEG_CHEM, 2.0)
    for lane, weight in [(POS_CHEM, 4.0), (NEG_CHEM, -4.0)]:
        add('Ingest', HIGH[0], lane, weight)
    # Recovery doesn't prevent eating in place. No social-Grab prior.
    for pair, weight in [(HIGH[5], .75), (HIGH[9], .25), (REST, -.5), (ESCAPE, -.5)]:
        add('Inspect', pair, OBJECT, weight)
    add('Other', HIGH[4], VOCAL, .25)
    # Novelty/play lanes44/45 are zero until honest individual evidence exists.
    return result

def scalar_matched_step(signal, reference, branch_signal, branch_reference,
                        shared_background, projection_gain=.5, leak=.5):
    """Math-only unit-test target, not production neural evaluation."""
    assert 0 <= leak <= 1 and projection_gain >= 0
    return ((1-leak)*signal + leak*tanh(shared_background + projection_gain*branch_signal),
            (1-leak)*reference + leak*tanh(shared_background + projection_gain*branch_reference))

def candidate_delta(state, family, features):
    return sum(state.get(r.source, 0.0) * features[r.lane] * r.weight
               for r in readouts() if r.family == family)

def manifest():
    bs, rs = branches(), readouts()
    return {
        'selector': SELECTOR, 'status': 'source_only_not_admitted_not_executed',
        'existing_neuron_addresses_unchanged': [0, 2047],
        'new_neuron_range': [2048, 2111], 'active_new_neurons': 52,
        'reserved_new_neurons': list(range(2100, 2112)),
        'pairs': [asdict(p) for p in ALL_PAIRS],
        'drive_setpoints': dict(zip(DRIVES, SETPOINTS)),
        'new_afferent_contract': 'signal=drive; reference=declared setpoint; BOTH receive the SAME interoceptive/regional transform and offset; neither reads hidden world truth',
        'matched_dynamics': {'activation': 'Tanh', 'leak': .5, 'bias': 0.0,
             'homeostatic_gain': 0.0, 'metabolic_tracking_and_cost': 'retain_and_charge',
             'initial_activation': 0.0, 'initial_metabolic_load': 0.0,
             'plasticity': 'REQUIRES explicit versioned fixed-parameter admission for all new paired parameters across every learning writer; alpha0/genetic0 is NOT a freeze; ordinary learning elsewhere retained'},
        'new_branch_semantics': 'projection_gain * tanh(max(sum(inputs)-threshold,0))*output_gain; shared local threshold/background then applied to matched neuron pair',
        'new_branches': [asdict(b) for b in bs],
        'new_arbitration_readout_head': [asdict(r) for r in rs],
        'extra_cost_before_tile_padding': {'neurons': 64, 'sensor_assignments': 22,
              'branches': len(bs), 'branch_inputs': sum(len(b.inputs) for b in bs),
              'decoder_rows': len(rs)},
        'no_default_promotion': True, 'class5_alone_admits_this': False,
        'native_execution': False, 'legacy_weights_or_optimizer_changed': False,
        'required_new_identity': ['capacity', 'foundation', 'encoder', 'dynamics',
                                 'branch_plan', 'decoder_head', 'execution_budget'],
        'staging_dependency': 'current derived object/chem/thermal/peer features only; new affine reference and readout heads still need CPU/WGSL implementation and tests',
        'important_limits': ['independent T1 motor sampling remains stochastic and can still select moving Rest',
             'high ATP is availability, not distress', 'temperature is unsigned bodily stress',
             'unbound extension1 has no behavior', 'peer is not mate eligibility',
             'negative taste is not measured poison', 'no novelty/play inference invented'],
    }

def mediation_manifest(include_legacy=False):
    """Exact source-only controller -> motor-pair -> decoder construction.

    M0 mediates every new output; legacy motor readouts remain separately
    measured. M1 also mediates the full old action-proposal bank. Neither is
    admitted by an existing runtime, and neither claims old function identity.
    """
    direct = readouts()
    keys = sorted(set((r.family, r.source) for r in direct[::2]),
                  key=lambda x: (FAMILIES.index(x[0]), x[1]))
    assert len(keys) == 20
    execution = {key: Pair('execute_' + key[0] + '_' + str(key[1]),
                           2100+2*i, 2101+2*i) for i, key in enumerate(keys)}
    extra_branches, mediated = [], []
    for (family, source), dst in execution.items():
        src = next(p for p in ALL_PAIRS if p.signal == source)
        extra_branches += [Branch(dst.signal, expand((src, 4.0))),
                           Branch(dst.reference, expand((src, 0.0)))]
    for a, b in zip(direct[::2], direct[1::2]):
        dst = execution[(a.family, a.source)]
        mediated += [Readout(a.family, dst.signal, a.lane, a.weight),
                     Readout(b.family, dst.reference, b.lane, b.weight)]
    result = manifest()
    result.update({
        'selector': SELECTOR + ('_M1AllActionProposalMediation' if include_legacy else '_M0NewPathMediation'),
        'variant': 'M1' if include_legacy else 'M0',
        'old_proposal_path': 'legacy readout stays an explicitly measured parallel path' if not include_legacy else 'all 128 legacy action proposal sources enter dedicated fresh signed execution pairs; legacy head retained but non-emitting under explicit routing policy',
        'active_new_neurons': 92, 'new_neuron_range': [2048, 2143],
        'reserved_new_neurons': list(range(2140,2144)),
        'execution_pairs': [dict(family=f, controller_signal=s, **asdict(p))
                            for (f,s),p in execution.items()],
        'new_branches': [asdict(b) for b in branches()+extra_branches],
        'new_arbitration_readout_head': [asdict(r) for r in mediated],
        'direct_controller_readouts_active': False,
        'extra_cost_before_tile_padding': {'neurons':96, 'sensor_assignments':22,
             'branches':70, 'branch_inputs':184, 'recurrent_copy_rows':0,
             'decoder_rows':50},
        'same_tick_latency_warning': 'one more microstep dependency than direct supervision; do not add an unpriced hidden settle pass',
        'legacy_neural_outputs_exactly_preserved': False,
        'lifetime_contract': 'Every new paired dynamics/branch/readout/copy parameter must be explicitly fixed across ordinary somatic, replay, consolidation, structural and optimizer writes for initial qualification, or updated with a versioned priced symmetric-parameter rule. Zero genetic weight or alpha is not a lifetime freeze. Verify offsets and restore; learning elsewhere continues.',
    })
    if include_legacy:
        legacy = [dict(family=FAMILIES[i//16], local_motor=i%16,
                      proposal_source=1728+i, signal=2144+2*i,
                      reference=2145+2*i) for i in range(128)]
        result.update({
            'active_new_neurons':348, 'new_neuron_range':[2048,2399],
            'legacy_execution_pairs':legacy,
            'legacy_copy_recurrent_rows':[row for p in legacy for row in (
                dict(source=p['proposal_source'], target=p['signal'], weight=1.0),
                dict(source=p['proposal_source'], target=p['reference'], weight=0.0))],
            'legacy_signed_readout_rule': {
                'expected_old_rows':4352,
                'input_lanes':list(range(24))+list(range(36,46)),
                'source_count':128,
                'new_rows':8704,
                'for_each_exact_old_family_lane_motor_coefficient_w':
                    'append w on matching execution signal and -w on matching execution reference; resolve/validate persistent coordinates, never guessed packed IDs',
                'initial_parameter_policy':
                    'fixed mirrored copy for mechanism qualification; this is not preservation of old decoder learning. Later learning requires explicitly tied updates and priced/versioned optimizer/eligibility semantics',
                'old_bank':
                    'retained for provenance and diagnostics, but excluded from decision emission by route/head policy, not by zero genetic weights',
            },
            'extra_cost_before_tile_padding':{'neurons':352, 'sensor_assignments':22,
                 'branches':70, 'branch_inputs':184, 'recurrent_copy_rows':256,
                 'decoder_rows':8754},
            'allocated_synapse_rows_if_class5_base34048':43058,
            'legacy_transfer_warning':
                'ordinary +1 copy receives projection gain and tanh/leak/background; signed pair difference carries the old proposal but rescales, delays and may saturate it. Measure gain, sign and fidelity rather than claiming identity. No semantic compression or guessed average of old neurons.',
            'remaining_nonmotor_bypasses':
                'innate scalar, memory and cognitive candidate terms, and independent speech-token decoder remain separately audited; M1 is action-neuron mediation, not a claim all decision components are mediated',
        })
    return result


def materialize_legacy_readouts(old_rows):
    """Constructor for exact M1 mirrored rows from an authorized fresh asset.

    old_rows: each dict contains family, local_motor, lane, weight, and the
    caller's verified persistent-address provenance. No file is opened here.
    """
    expected = {(f, m, lane) for f in FAMILIES for m in range(16)
                for lane in list(range(24))+list(range(36,46))}
    seen, out = set(), []
    for row in old_rows:
        key = (row['family'], row['local_motor'], row['lane'])
        if key not in expected or key in seen or not row.get('persistent_address'):
            raise ValueError('Legacy action coordinate missing, duplicate, invalid, or unverified')
        seen.add(key)
        i = FAMILIES.index(row['family'])*16 + row['local_motor']
        weight = float(row['weight'])
        if not (-float('inf') < weight < float('inf')):
            raise ValueError('Nonfinite coefficient')
        out += [Readout(row['family'], 2144+2*i, row['lane'], weight),
                Readout(row['family'], 2145+2*i, row['lane'], -weight)]
    if seen != expected:
        raise ValueError('Incomplete legacy action bank')
    return out

if __name__ == '__main__':
    print(json.dumps(manifest(), indent=2))
