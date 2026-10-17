"""Build an exact-body CPU sync component probe from Cargo JSONL artifacts.

Run from the repository root with artifact JSONL and output executable paths.
The minimal resident retains the three fields read/written by the exact helpers;
full app-resident equality/error/rollback is covered by the actual app tests.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

artifacts, executable = map(Path, sys.argv[1:])
root = Path.cwd()
app_path = root / 'crates/alife_game_app/src/gpu_live_runtime.rs'
legacy_path = root / 'crates/alife_game_app/src/gpu_live_runtime/resident_sync_tests.rs'
app = app_path.read_text()
legacy = legacy_path.read_text()
start = 'fn synchronize_resident_from_record('
end = '\n#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]\nstruct AuthorityAdvanceTiming'
helpers = app[app.index(start):app.index(end, app.index(start))]
frozen = legacy[legacy.index('fn legacy_synchronize_resident_from_record('):legacy.index('\nfn fixture()')]
base = subprocess.check_output(['git', 'show', '69f00c05:crates/alife_game_app/src/gpu_live_runtime.rs'], text=True)
base_helpers = base[base.index(start):base.index(end, base.index(start))]
assert ''.join(base_helpers.split()) == ''.join(frozen.replace('legacy_', '').split()), 'Frozen baseline differs'
source = r'''
use alife_core::*;
use alife_world::{HeadlessWorld, HeadlessScenarioBuilder, WorldOrganismRecord};
use std::{alloc::{GlobalAlloc, Layout, System}, collections::BTreeMap, hint::black_box,
    sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed}, time::Instant};
struct Counter;
static COUNT: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNT.load(Relaxed) { ALLOCS.fetch_add(1, Relaxed); BYTES.fetch_add(l.size() as u64, Relaxed); }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { unsafe { System.dealloc(p, l) } }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if COUNT.load(Relaxed) { ALLOCS.fetch_add(1, Relaxed); BYTES.fetch_add(n as u64, Relaxed); }
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator] static ALLOCATOR: Counter = Counter;
#[derive(Debug, Clone, PartialEq)]
struct ResidentCognition {
    genome: BrainGenome,
    homeostasis: HomeostaticSnapshot,
    development: DevelopmentState,
}
HELPERS
FROZEN
fn fixture(population: usize, inherited_asset: bool) -> (HeadlessWorld, BTreeMap<u64, ResidentCognition>) {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let asset = FoundationWeightAsset::builtin_n2048_v1(profile).unwrap();
    assert_eq!(asset.weights().len(), 32768);
    let manifest = asset.manifest();
    let identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(), manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(), BrainCapacityClass::N2048_ID,
    ).unwrap();
    let mut builder = HeadlessScenarioBuilder::new(77112);
    for raw in 1..=population as u64 {
        builder = builder.agent(&format!("agent-{raw}"), OrganismId(raw), Vec3f::new(raw as f32, 0.0, 0.0));
    }
    let mut world = builder.build().unwrap();
    let mut residents = BTreeMap::new();
    for (id, entity) in world.organism_entity_ids() {
        let mut genome = CreatureGenome::early_mammal_founder(77112 + id.raw(), identity).unwrap();
        if inherited_asset { genome = genome.with_n2048_foundation_candidate(asset.clone()).unwrap(); }
        let phenotype = genome.express().unwrap();
        let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        let resident = ResidentCognition {
            genome: phenotype.brain_genome.clone(), homeostasis: biology.homeostasis,
            development: phenotype.development_state_at(Tick::ZERO).unwrap(),
        };
        let record = WorldOrganismRecord::new(id, entity, genome, phenotype, biology, Tick::ZERO).unwrap();
        record.validate_contract().unwrap(); // Warm the same private immutable proof in both modes.
        world.register_organism_record(record).unwrap();
        residents.insert(id.raw(), resident);
    }
    (world, residents)
}
fn run(world: &HeadlessWorld, residents: &mut BTreeMap<u64, ResidentCognition>, old: bool, repeats: usize) {
    for _ in 0..repeats {
        for (&raw, resident) in residents.iter_mut() {
            let record = world.organism_registry().get(OrganismId(raw)).unwrap();
            if old { legacy_synchronize_resident_from_record(black_box(resident), black_box(record), Tick::ZERO).unwrap(); }
            else { synchronize_resident_from_record(black_box(resident), black_box(record), Tick::ZERO).unwrap(); }
        }
        if old { legacy_synchronize_residents_from_world(black_box(world), black_box(residents), Tick::ZERO).unwrap(); }
        else { synchronize_residents_from_world(black_box(world), black_box(residents), Tick::ZERO).unwrap(); }
    }
}
fn main() {
    for inherited_asset in [false, true] {
        for population in [8, 50] {
            let (world, residents) = fixture(population, inherited_asset);
            let before = world.canonical_signature_digest().unwrap();
            let mut expected = None;
            for old in [true, false] {
                let mut owned = residents.clone();
                ALLOCS.store(0, Relaxed); BYTES.store(0, Relaxed); COUNT.store(true, Relaxed);
                run(&world, &mut owned, old, 1);
                COUNT.store(false, Relaxed);
                println!("ALLOCATION inherited_asset={} population={} old={} calls={} requested_bytes={}",
                    inherited_asset, population, old, ALLOCS.load(Relaxed), BYTES.load(Relaxed));
                if old { expected = Some(owned); } else { assert_eq!(expected.as_ref().unwrap(), &owned); }
            }
            if std::env::var_os("ALIFE_SYNC_ALLOCATION_ONLY").is_some() {
                assert_eq!(world.canonical_signature_digest().unwrap(), before);
                continue;
            }
            let timed = |old| {
                let mut owned = residents.clone();
                let started = Instant::now(); run(&world, &mut owned, old, 20);
                (started.elapsed().as_nanos(), owned)
            };
            for _ in 0..2 { let a = timed(true); let b = timed(false); assert_eq!(a.1, b.1); }
            for sample in 0..9 {
                let (old, new) = if sample % 2 == 0 { (timed(true), timed(false)) }
                    else { let new = timed(false); (timed(true), new) };
                assert_eq!(old.1, new.1);
                println!("TIMING inherited_asset={} population={} sample={} old_first={} old_ns={} new_ns={} repeats=20 exact_residents={}",
                    inherited_asset, population, sample, sample % 2 == 0, old.0, new.0, new.1.len());
                black_box((old, new));
            }
            assert_eq!(world.canonical_signature_digest().unwrap(), before);
        }
    }
}
'''.replace('HELPERS', helpers).replace('FROZEN', frozen)
source_path = executable.with_suffix('.rs')
source_path.write_text(source)
libs = {}
for line in artifacts.read_text().splitlines():
    item = json.loads(line)
    if item.get('reason') == 'compiler-artifact' and 'lib' in item['target']['kind']:
        for name in item['filenames']:
            if name.endswith('.rlib'):
                libs[item['target']['name']] = name
command = ['rustc', '--edition=2021', '-O', str(source_path), '-L',
           'dependency=' + str(Path(libs['alife_core']).parent), '-o', str(executable)]
for name in ['alife_core', 'alife_world']:
    command += ['--extern', name + '=' + libs[name]]
subprocess.run(command, check=True)
hash_file = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
print(json.dumps({
    'source_sha256': hash_file(source_path), 'executable_sha256': hash_file(executable),
    'app_sha256': hash_file(app_path), 'frozen_helpers_sha256': hashlib.sha256(frozen.encode()).hexdigest(),
    'linked_libraries': {name: {'path': libs[name], 'sha256': hash_file(libs[name])}
                         for name in ['alife_core', 'alife_world']},
    'command': command,
}, indent=2))
