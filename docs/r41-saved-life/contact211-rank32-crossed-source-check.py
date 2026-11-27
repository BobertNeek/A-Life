"""Validate, reversibly apply and re-apply only the crossed research source overlay."""
from hashlib import sha256
from pathlib import Path
import json,os,re,subprocess,sys

root = Path.cwd()
pin_leaf, patch_leaf, tree_leaf = map(Path, sys.argv[1:])
digest = lambda p: sha256(p.read_bytes()).hexdigest()
assert digest(pin_leaf) == os.environ['PIN_CROSSED_LEAVES']
assert digest(patch_leaf) == os.environ['PIN_CROSSED_PATCH']
pin = json.loads(pin_leaf.read_bytes())
assert pin['full_hosted_parent_commit'] == os.environ['PIN_RANK_COMMIT']
assert pin['forward_sha256'] == os.environ['PIN_CROSSED_PATCH']
app = 'crates/alife_game_app/'
ret = app + 'src/foundation_training_cycle/saved_life_retention/'
allowed = {app + 'Cargo.toml', ret + 'contact211_parity.rs', ret + 'experiments.rs',
           ret + 'language_pilot.rs', app + 'src/gpu_live_runtime/contact211_f0_witness.rs'}
before = pin['five_source_only_before_sha256']
after = pin['five_source_only_after_sha256']
assert set(before) == set(after) == allowed
names = [re.fullmatch(r'diff --git a/(\S+) b/(\S+)', s)
         for s in patch_leaf.read_text().splitlines() if s.startswith('diff --git ')]
assert len(names) == 5 and all(x and x[1] == x[2] for x in names) and {x[1] for x in names} == allowed

def tree():
    walked = list(root.rglob('*'))
    assert not any(p.is_symlink() for p in walked)
    assert not any(p.name in {'rows.jsonl','Current.alife-checkpoint','contact211-rank32.weights.v1.json',
        'CONTACT-LEXICAL-WEIGHTS.v1.json','contact211-stock-reference.v1.json'} for p in walked)
    return {p.relative_to(root).as_posix():digest(p) for p in walked if p.is_file()}

def apply(*opts):
    subprocess.run(['git','apply','--whitespace=error-all',*opts,str(patch_leaf)],cwd=root,check=True)

old = json.loads(tree_leaf.read_bytes())
assert tree() == old and all(old.get(p) == k for p,k in before.items())
apply('--check')
apply()
new = tree()
assert new == old | after
apply('--reverse','--check')
apply('--reverse')
assert tree() == old
apply()
assert tree() == new
mode = (root/(ret+'experiments.rs')).read_text()
lex = (root/(ret+'language_pilot.rs')).read_text()
assert '#[cfg(not(feature = "contact211-rank32-crossed"))]\n    if version.schema == 8 { return Err(' in mode
assert 'if !crossed && (disclosed["layout"].as_str() != Some("a")' in mode
assert '!crossed_rank32 && matches!(tokens.as_slice(), [9, 1] | [9, 14] | [14, 1] | [6, 1])' in lex
tree_leaf.write_text(json.dumps(new,sort_keys=True)+'\n')
print('crossed_source=exact_five_leaf_apply_inverse_and_shaders_lock_frozen')
print('crossed_feature_off_guard=compile_time_refusal_before_checkpoint_access')
print('crossed_source_scope=CPU_tests_only; no_private_data_or_evaluator_execution')
