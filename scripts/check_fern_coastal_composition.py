"""Fixed union of approved Fern and CoastalShrub upgrades, stdlib CPU only.

Independent historical profiles/checkers remain byte-retained. This successor
checks both geometry contracts and the exact art/world/source/registration union.
It establishes no live-game, visual acceptance, GPU, FPS or training result.
"""
if not __debug__:
    raise RuntimeError('Combined plant checks require normal Python mode')

import argparse
import copy
import json
from pathlib import Path

import check_fern_fronds as fern
import check_coastal_leaf_sprays as coastal

ROOT = Path(__file__).resolve().parents[1]
HERE = fern.HERE
MANIFEST = fern.MANIFEST
SPECIFICATION = coastal.SPECIFICATION
HISTORICAL_PROFILE = Path('scripts/fern-coastal-composition-profile.json')
HISTORICAL_PROFILE_SHA256 = '6bc1ed02fc8805bd8613717267c50b0a784b031b8fcefa485200d1506c15cb52'
PROFILE = Path('scripts/fern-coastal-preview-rotation-profile.json')
PROFILE_SHA256 = '54636cd97fab71ce79b523210f40a1d620f5fb3983388c623a6c6b62294c27dc'
MAIN_INTEGRATION = Path('scripts/fern-coastal-main-integration-profile.json')
MAIN_INTEGRATION_SHA256 = 'f5ba58ea4aa7dd15007cd0136cca18cedc5637e44ead60a8b5da45fdafc9f829'
FROZEN = {
    HISTORICAL_PROFILE.as_posix(): HISTORICAL_PROFILE_SHA256,
    fern.PROFILE.as_posix(): fern.PROFILE_SHA256,
    coastal.PROFILE.as_posix(): coastal.PROFILE_SHA256,
    fern.PREDECESSOR.as_posix(): fern.PREDECESSOR_SHA256,
    fern.CONSOLIDATED.as_posix(): fern.CONSOLIDATED_SHA256,
    'scripts/check_fern_fronds.py': '9f7cc4e197ea9701351f3b84adee25bba03a855788fea8fde3a9eb66867231a1',
    'scripts/check_coastal_leaf_sprays.py': 'f389cbb5d7948d1aa4d407dc701bcfb87c9c2d4d713c3d24cb0fff271a2d737e',
}
SUCCESSOR_SOURCES = {
    'scripts/test_fern_coastal_composition.py',
    'scripts/check_fern_coastal_export_order.py',
    '.github/workflows/ci.yml',
    'scripts/render_coastal_leaf_sprays.py',
    'scripts/render_fern_fronds_patch.py',
    'scripts/check_authored_preview_rotation.py',
    'scripts/test_authored_preview_rotation.py',
}
# Only these two historically pinned executables change in this successor.
RENDERER_CORRECTIONS = {
    'scripts/render_coastal_leaf_sprays.py': {
        'historical_sha256': '995802c09efb276e0295aba76810abeecd5026fb1108a642f71a26f2f6d9d1ed',
        'corrected_sha256': 'c4b5edb65856ce7a34d526958ff8f8c353e048ab6760160b5a2ac405f5e2b91a'},
    'scripts/render_fern_fronds_patch.py': {
        'historical_sha256': 'fd28e3174657d56595b67c935e6e654034a73e7ea8e5c39ab3dd01e3bed6ad26',
        'corrected_sha256': 'bd533a774db83863e42bd65c8f0d635a1cbd8a717419034b24b0b62fb905d466'},
}
SHARED = {(HERE / 'README.md').as_posix(), SPECIFICATION.as_posix(), MANIFEST.as_posix()}
RUNTIMES = {fern.RUNTIME, coastal.RUNTIME}
ADDED_ART = fern.ADDED_ART | coastal.ADDED_ART
FERN_RUNTIME_SHA256 = '00adc40d44b34bb99e81407efb3944ab53c1c48a2851c883df6ca784d4e57aa3'
FERN_NATIVE_SHA256 = 'dac2ac84d94bcc15127e9bbaf42e683f2eb2d7dea99ee3d965bfbad2c2379cc5'


def merge_pins(*mappings):
    result = {}
    for mapping in mappings:
        for name, digest in mapping.items():
            assert name not in result or result[name] == digest, ('Conflicting source pins', name)
            result[name] = digest
    return result


def load_main_integration(root):
    raw = coastal.safe_path(root, PROFILE, 'Profile').read_bytes()
    assert coastal.sha(raw) == PROFILE_SHA256, 'Fixed combined profile changed'
    profile = json.loads(raw)
    raw = coastal.safe_path(root, MAIN_INTEGRATION, 'Main integration profile').read_bytes()
    assert coastal.sha(raw) == MAIN_INTEGRATION_SHA256, 'Fixed main integration profile changed'
    integration = json.loads(raw)
    assert set(integration) == {'profile', 'base_commit', 'base_tree', 'historical_profile',
                                'workflow_transition', 'integration_test'}, 'Integration profile inventory changed'
    assert integration['historical_profile'] == {
        'path': PROFILE.as_posix(), 'sha256': PROFILE_SHA256}, 'Integration historical profile changed'
    transition = integration['workflow_transition']
    assert set(transition) == {'path', 'historical_sha256', 'current_sha256'}, 'CI transition inventory changed'
    assert transition['path'] == '.github/workflows/ci.yml', 'Only the current CI source may be superseded'
    probe = integration['integration_test']
    assert set(probe) == {'path', 'sha256'}, 'Integration test inventory changed'
    assert probe['path'] == 'scripts/test_fern_coastal_main_integration.py', 'Integration test path changed'
    coastal.validate_source(root, probe['path'], probe['sha256'])
    return profile, integration


def apply_main_integration(profile, integration):
    transition = integration['workflow_transition']
    assert profile['sources'][transition['path']] == transition['historical_sha256'], 'Historical CI source pin changed'
    current = copy.deepcopy(profile)
    current['sources'][transition['path']] = transition['current_sha256']
    return current


def load_profiles(root):
    historical, integration = load_main_integration(root)
    profile = apply_main_integration(historical, integration)
    assert profile['historical_combined_profile'] == {
        'path': HISTORICAL_PROFILE.as_posix(), 'sha256': HISTORICAL_PROFILE_SHA256}, \
        'Historical combined profile lineage changed'
    assert profile['independent_snapshot_profiles'] == {
        fern.PROFILE.as_posix(): fern.PROFILE_SHA256,
        coastal.PROFILE.as_posix(): coastal.PROFILE_SHA256}, 'Combined profile lineage changed'
    for name, digest in FROZEN.items():
        coastal.validate_source(root, name, digest)
    fp = json.loads((root / fern.PROFILE).read_text())
    cp = json.loads((root / coastal.PROFILE).read_text())
    before = json.loads((root / fern.PREDECESSOR).read_text())
    consolidated = json.loads((root / fern.CONSOLIDATED).read_text())
    assert fp['predecessor_profile_sha256'] == cp['predecessor_profile_sha256'] == fern.PREDECESSOR_SHA256
    return profile, fp, cp, before, consolidated


def validate_sources(profile, fp, cp, before, consolidated):
    # Also exercise the Coastal predecessor's explicit imported-helper inventory.
    coastal.validate_profile_sources(cp, before, consolidated)
    retained = merge_pins(fp['retained_sources'], fp['sources'],
                          cp['retained_sources'], cp['sources'], FROZEN)
    assert profile['renderer_corrections'] == RENDERER_CORRECTIONS, 'Unreviewed renderer correction'
    for name, correction in RENDERER_CORRECTIONS.items():
        assert retained.pop(name) == correction['historical_sha256'], 'Historical renderer pin changed'
        assert profile['sources'].get(name) == correction['corrected_sha256'], 'Corrected renderer pin changed'
    assert profile['retained_sources'] == retained, 'Combined retained source inventory changed'
    assert set(profile['sources']) == SUCCESSOR_SOURCES, 'Combined successor source inventory changed'
    assert not set(retained) & set(profile['sources']), 'Combined source inventories overlap'
    return merge_pins(retained, profile['sources'])


def validate_retention(profile, fp, cp, before):
    # Preserve each independent snapshot's invariants before composing them.
    coastal.validate_retention(cp, before)
    old = coastal.inventory(before['expected_art_files'])
    ffiles = coastal.inventory(fp['expected_art_files'])
    cfiles = coastal.inventory(cp['expected_art_files'])
    assert set(ffiles) - set(old) == fern.ADDED_ART and not set(old) - set(ffiles)
    for name in set(old) - fern.ALLOWED_CHANGES:
        assert ffiles[name] == old[name], ('Independent Fern retention changed', name)
    assert ffiles[fern.RUNTIME]['sha256'] == FERN_RUNTIME_SHA256
    assert ffiles[fern.SOURCE.as_posix()]['sha256'] == FERN_NATIVE_SHA256
    assert ffiles[fern.BASELINE.as_posix()] == dict(old[fern.RUNTIME], path=fern.BASELINE.as_posix())
    actual = coastal.inventory(profile['expected_art_files'])
    assert set(actual) == set(old) | ADDED_ART, 'Combined art path retention changed'
    for name in set(old) - SHARED - RUNTIMES:
        assert actual[name] == old[name], ('Unrelated combined art record changed', name)
    for name in {fern.RUNTIME} | fern.ADDED_ART:
        assert actual[name] == ffiles[name], ('Approved Fern art changed', name)
    for name in {coastal.RUNTIME} | coastal.ADDED_ART:
        assert actual[name] == cfiles[name], ('Approved CoastalShrub art changed', name)
    return actual


def validate_specification(root, profile, cp):
    baseline = cp['baseline_specification']
    assert coastal.canonical_sha(baseline) == coastal.BASELINE_SPECIFICATION_SHA256
    expected = copy.deepcopy(baseline)
    fspec = profile['fern_specification']
    assert fspec == dict(baseline['assets']['Fern'], near_triangles=700,
                         far_triangles=294, overview_triangles=154,
                         editable_override=fern.SOURCE.name,
                         export_recipe='scripts/build_fern_fronds.py', override_blender='4.3.2'), \
        'Combined Fern specification changed'
    expected['assets']['Fern'] = fspec
    expected['assets']['CoastalShrub'] = cp['coastal_specification']
    actual = json.loads(coastal.safe_path(root, SPECIFICATION, 'Specification').read_text())
    assert actual == expected, 'Combined specification or unrelated asset changed'


def expected_manifest(root, profile, fp, cp, before, consolidated):
    expected = coastal.predecessor.expected_manifest(before, consolidated, root)
    old = {entry['local_path']: copy.deepcopy(entry) for entry in expected['entries']}
    overrides = {fern.RUNTIME: fp['manifest_override'], coastal.RUNTIME: cp['manifest_override']}
    assert profile['manifest_overrides'] == overrides, 'Combined manifest overrides changed'
    for name, override in overrides.items():
        assert set(override) == {'digest', 'size_bytes', 'generator'}
    for entry in expected['entries']:
        if entry['local_path'] in overrides:
            entry.update(overrides[entry['local_path']])
    changed = {entry['local_path'] for entry in expected['entries'] if entry != old[entry['local_path']]}
    assert changed == RUNTIMES, 'Expected exactly two approved manifest deltas'
    return expected


def check(root=ROOT):
    root = Path(root).resolve()
    profile, fp, cp, before, consolidated = load_profiles(root)
    sources = validate_sources(profile, fp, cp, before, consolidated)
    helper_roots = [ROOT] + [Path(module.__file__).resolve().parents[1]
                            for module in (fern, coastal, coastal.predecessor,
                                           coastal.island_art, coastal.island_canopies)]
    coastal.validate_source_trees(root, sources, helper_roots)
    records = validate_retention(profile, fp, cp, before)
    coastal.validate_files(root, profile['expected_art_files'])
    validate_specification(root, profile, cp)
    expected = expected_manifest(root, profile, fp, cp, before, consolidated)
    actual = json.loads(coastal.safe_path(root, MANIFEST, 'Manifest').read_text())
    coastal.validate_manifest_records(root, actual, expected)
    placements = coastal.validate_placements(root)
    fern_lods = fern.geometry(root / fern.RUNTIME, root / fern.BASELINE)
    assert [row['triangles'] for row in fern_lods] == [700, 294, 154]
    assert [row['vertices'] for row in fern_lods] == [952, 441, 231]
    coastal_lods = coastal.geometry(root / coastal.RUNTIME, root / coastal.BASELINE)
    broadleaf = coastal.predecessor.geometry(root / HERE / 'Broadleaf.glb')
    return {'result': 'PASS', 'profile': profile['profile'],
            'main_integration_profile': MAIN_INTEGRATION.as_posix(),
            'main_integration_sha256': MAIN_INTEGRATION_SHA256, 'fern_lods': fern_lods,
            'coastal_lods': coastal_lods, 'art_blobs_audited': len(records),
            'manifest_entries_valid': len(actual['entries']), 'other_manifest_records_retained': 46,
            'exact_approved_native_and_runtime_bytes_retained': True,
            'both_prior_runtime_snapshots_retained': True,
            'coastal_shrub_placements': placements,
            'coastal_l2_attributes_and_indices_retained': True,
            'all_other_art_and_world_bytes_retained': True,
            'historical_profiles_retained': True,
            'unchanged_historical_source_files_retained': True,
            'historical_renderer_source_pins_explicitly_superseded': True,
            'historical_combined_profile_retained': True,
            'renderer_corrections': RENDERER_CORRECTIONS,
            'combined_visual_composition_review': False,
            'retained_source_files_audited': len(sources),
            'retained_broadleaf_triangles': broadleaf['triangles'],
            'game_rendering_proof': False, 'physical_gpu_proof': False,
            'whole_workspace_acceptance': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    text = json.dumps(check(args.root), indent=2) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
    print(text, end='')
