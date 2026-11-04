//! Source-derived, disposable initial-state experiments. Never mutates a parent.
use super::*;
use alife_gpu_backend::{GpuLiveTopologyCheckpointV1, GpuPhenotypeUpload, GpuV11Checkpoint};
use alife_world::persistence::PortableDualWeightBankV1;

const VARIANTS: [&str; 11] = [
    "pc_retained",
    "native_dendrites",
    "no_dendrites",
    "reset_predictor",
    "reset_lifetime",
    "reset_lifetime_decoder",
    "reset_lifetime_recurrent",
    "reset_fast",
    "reset_somatic",
    "reset_somatic_predictor",
    "edge_ablated",
];
fn zero_bank(bank: &mut PortableDualWeightBankV1) -> Result<()> {
    bank.bank_0_bits.fill(0f32.to_bits());
    bank.bank_1_bits.fill(0f32.to_bits());
    bank.canonical_digest = bank.recompute_canonical_digest()?;
    bank.validate()?;
    Ok(())
}
fn zero_lifetime_subset(
    next: &mut CpuRetainedNeuralInheritance,
    p: &BrainPhenotype,
    decoder: bool,
) -> Result<()> {
    let map = next.canonical_to_live(p)?;
    for (canonical, synapse) in p.synapses().iter().enumerate() {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(_)) == decoder {
            let slot = map[canonical];
            next.lifetime.bank_0_bits[slot] = 0f32.to_bits();
            next.lifetime.bank_1_bits[slot] = 0f32.to_bits();
        }
    }
    next.lifetime.canonical_digest = next.lifetime.recompute_canonical_digest()?;
    next.lifetime.validate()?;
    Ok(())
}
fn edge_ablation(
    source: &CpuRetainedNeuralInheritance,
    p: &BrainPhenotype,
) -> Result<CpuRetainedNeuralInheritance> {
    let old_map = source.canonical_to_live(p)?;
    let mut checkpoint = source.topology.v11_checkpoint.clone();
    // Keep candidate evidence/configuration, remove actual inherited connections.
    // Native validation and recompilation reject any inconsistent changed copy.
    let mut structural = serde_json::to_value(&checkpoint.structural)?;
    structural["connections"] = json!({});
    checkpoint.structural = serde_json::from_value(structural)?;
    checkpoint.sparse_spans.clear();
    checkpoint.pending_lifetime_synapse = None;
    let topology = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(p, checkpoint)?;
    if topology.total_synapse_count as usize != p.synapses().len() {
        return Err("edge ablation did not recover the canonical slot count".into());
    }
    let upload = GpuPhenotypeUpload::try_from(p)?;
    let new_map = &upload.canonical_to_local_synapse;
    let remap = |old: &PortableDualWeightBankV1| -> Result<PortableDualWeightBankV1> {
        let mut new = old.clone();
        new.synapse_count = topology.total_synapse_count;
        new.bank_0_bits = vec![0; new.synapse_count as usize];
        new.bank_1_bits = new.bank_0_bits.clone();
        for (canonical, &old_slot) in old_map.iter().enumerate() {
            let slot = new_map[canonical] as usize;
            new.bank_0_bits[slot] = old.bank_0_bits[old_slot];
            new.bank_1_bits[slot] = old.bank_1_bits[old_slot];
        }
        new.canonical_digest = new.recompute_canonical_digest()?;
        new.validate()?;
        Ok(new)
    };
    let result = CpuRetainedNeuralInheritance {
        lifetime: remap(&source.lifetime)?,
        fast: remap(&source.fast)?,
        topology,
        predictor: source.predictor.clone(),
    };
    let admitted = result.canonical_to_live(p)?;
    for (canonical, &slot) in admitted.iter().enumerate() {
        let old_slot = old_map[canonical];
        for (old, new) in [
            (&source.lifetime, &result.lifetime),
            (&source.fast, &result.fast),
        ] {
            if old.bank_0_bits[old_slot] != new.bank_0_bits[slot]
                || old.bank_1_bits[old_slot] != new.bank_1_bits[slot]
            {
                return Err("edge ablation changed a canonical mutable coordinate".into());
            }
        }
    }
    Ok(result)
}
pub(super) fn modify(
    role: &str,
    source: &CpuRetainedNeuralInheritance,
    p: &BrainPhenotype,
) -> Result<CpuRetainedNeuralInheritance> {
    let original = serde_json::to_vec(source)?;
    let mut next = source.clone();
    let mut checkpoint = source.topology.v11_checkpoint.clone();
    match role {
        "pc_retained" => (),
        "native_dendrites" => {
            checkpoint.dendritic_branches =
                GpuV11Checkpoint::canonical_for_phenotype(p)?.dendritic_branches;
            next.topology = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(p, checkpoint)?;
        }
        "no_dendrites" => {
            checkpoint.dendritic_branches = Default::default();
            next.topology = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(p, checkpoint)?;
        }
        "reset_predictor" => {
            next.predictor = alife_runtime::canonical_preparation::predictor_for_phenotype(p)?
        }
        "reset_lifetime" => zero_bank(&mut next.lifetime)?,
        "reset_lifetime_decoder" => zero_lifetime_subset(&mut next, p, true)?,
        "reset_lifetime_recurrent" => zero_lifetime_subset(&mut next, p, false)?,
        "reset_fast" => zero_bank(&mut next.fast)?,
        "reset_somatic" | "reset_somatic_predictor" => {
            zero_bank(&mut next.lifetime)?;
            zero_bank(&mut next.fast)?;
            if role == "reset_somatic_predictor" {
                next.predictor = alife_runtime::canonical_preparation::predictor_for_phenotype(p)?;
            }
        }
        "edge_ablated" => next = edge_ablation(source, p)?,
        _ => return Err("unsupported disposable ablation".into()),
    }
    next.canonical_to_live(p)?;
    if serde_json::to_vec(source)? != original {
        return Err("ablation modified its source".into());
    }
    Ok(next)
}
pub(super) fn models(
    request: &Request,
    pc: &alife_training::FoundationTrainingHandoffV1,
    cloud: &alife_training::FoundationTrainingHandoffV1,
    source: &CpuRetainedNeuralInheritance,
) -> Result<Vec<Model>> {
    let p =
        PhenotypeCompiler::compile_validated(&pc.compiler_inputs, &BrainCapacityClass::n2048())?;
    let mut models = Vec::new();
    for role in VARIANTS {
        if request.phase == "ablations" {
            let selected = if request.ablation_variant.as_deref() == Some("lifetime_refine") {
                matches!(
                    role,
                    "reset_lifetime" | "reset_lifetime_decoder" | "reset_lifetime_recurrent"
                )
            } else {
                !matches!(role, "reset_lifetime_decoder" | "reset_lifetime_recurrent")
            };
            if !selected {
                continue;
            }
        }
        if request.phase == "fresh_battery" && request.ablation_variant.as_deref() != Some(role) {
            continue;
        }
        let retained = modify(role, source, &p)?;
        let mut model = model_parent(role, pc, Some(retained.clone()))?;
        let component = |x: &CpuRetainedNeuralInheritance| -> Result<Value> {
            Ok(
                json!({"topology":hash(&serde_json::to_vec(&x.topology)?),"lifetime":hash(&serde_json::to_vec(&x.lifetime)?),"fast":hash(&serde_json::to_vec(&x.fast)?),"predictor":hash(&serde_json::to_vec(&x.predictor)?)}),
            )
        };
        model.identity = json!({"kind":"disposable-source-derived-initial-state-ablation","policy":pc.policy_version,"role":role,"source_components":component(source)?,"trial_components":component(&retained)?,"global_actor_value_and_all_Adam":"unchanged source heads","private_generation_and_bank_selector":"preserved; a zero operation resets both bit arrays, not history counters","new_life":"native newborn chemistry, memory, activations and eligibility; 64 native dendrites unless explicit no_dendrites","diagnostic_only":true,"source_immutable":true,"initial_acquired_connections":retained.topology.v11_checkpoint.structural.connection_count(),"edge_ablation_scope":"initial inherited connections only; ordinary subsequent structural learning is allowed"});
        models.push(model);
    }
    models.push(model_parent("pc_native_fresh", pc, None)?);
    models.push(model_parent("cloud_native_fresh", cloud, None)?);
    Ok(models)
}
pub(super) fn qualify_battery_request(request: &Request, executable_sha: &str) -> Result<()> {
    let variant = request
        .ablation_variant
        .as_deref()
        .ok_or("battery requires a selected variant")?;
    if !matches!(
        variant,
        "reset_fast"
            | "reset_lifetime"
            | "reset_lifetime_decoder"
            | "reset_lifetime_recurrent"
            | "reset_somatic"
            | "reset_somatic_predictor"
            | "reset_predictor"
            | "pc_native_fresh"
    ) {
        return Err("practical battery variant must preserve native dendrites and acquired structure, or be explicitly native fresh".into());
    }
    let previous = request
        .qualified_controls
        .as_ref()
        .ok_or("battery requires the actual ablation pilot")?;
    let native: Value = serde_json::from_slice(&fs::read(previous.join("native-admission.json"))?)?;
    let comparison: Value = serde_json::from_slice(&fs::read(previous.join("comparison.json"))?)?;
    if comparison["phase"] != "ablations"
        || comparison["parents_unchanged"] != true
        || native["pc_sha256"] != request.expected_pc_sha256
        || native["cloud_binding"]["tree_sha256"] != request.expected_cloud_tree_sha256
        || native["executable_sha256"] != executable_sha
    {
        return Err("ablation gate differs from selected parents or this executable".into());
    }
    let trials = comparison["trials"]
        .as_array()
        .ok_or("pilot lacks trials")?;
    let selected = trials
        .iter()
        .filter(|r| r["role"] == variant)
        .collect::<Vec<_>>();
    if selected.len() != 4
        || selected.iter().any(|r| {
            r["complete"] != true
                || r["alive"] != true
                || r["organisms"][0]["biology"]["body"]["injury"]
                    .as_f64()
                    .is_none_or(|v| v != 0.)
        })
        || selected
            .iter()
            .filter(|r| r["lesson_completed"] == true)
            .count()
            < 3
    {
        return Err("pilot does not support a safe promising battery candidate".into());
    }
    Ok(())
}
