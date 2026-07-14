//! Deterministic aligned lobe allocation for production phenotype compilation.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BrainGenome, DevelopmentState, LobeKind, LobeLayout, LobeRatioPlan, LobeRegion,
    ScaffoldContractError,
};

pub(super) fn compile_layout(
    genome: &BrainGenome,
    development: &DevelopmentState,
    neurons: u32,
) -> Result<LobeLayout, ScaffoldContractError> {
    const ESSENTIAL: [LobeKind; 4] = [
        LobeKind::SensoryGrounding,
        LobeKind::MetabolicDrive,
        LobeKind::MotorArbitration,
        LobeKind::HomeostaticRegulation,
    ];
    if matches!(genome.lobe_ratios, LobeRatioPlan::ClassDefault)
        && development.enabled_lobes.is_empty()
    {
        return LobeLayout::reference_for_neuron_count(neurons);
    }
    let reference = LobeLayout::reference_for_neuron_count(neurons)?;
    let mut weights = BTreeMap::new();
    for region in reference.iter_regions() {
        weights.insert(region.kind.raw(), region.len as f64 / neurons as f64);
    }
    match &genome.lobe_ratios {
        LobeRatioPlan::ClassDefault => {}
        LobeRatioPlan::RegistryRef(_) => return Err(compile_error()),
        LobeRatioPlan::InlineOverrides(rows) => {
            let mut seen = BTreeSet::new();
            for row in rows {
                if !seen.insert(row.lobe.raw()) {
                    return Err(compile_error());
                }
                weights.insert(row.lobe.raw(), f64::from(row.ratio.raw()));
            }
        }
    }
    if !development.enabled_lobes.is_empty()
        && ESSENTIAL
            .iter()
            .any(|kind| !development.enabled_lobes.contains(kind))
    {
        return Err(compile_error());
    }
    for kind in LobeKind::ALL {
        if !development.enabled_lobes.is_empty() && !development.enabled_lobes.contains(&kind) {
            weights.insert(kind.raw(), 0.0);
        }
    }
    if ESSENTIAL
        .iter()
        .any(|kind| weights.get(&kind.raw()).copied().unwrap_or(0.0) <= 0.0)
    {
        return Err(compile_error());
    }
    let block_count = neurons / 16;
    let sum = weights.values().sum::<f64>();
    if sum <= 0.0 {
        return Err(compile_error());
    }
    let enabled = LobeKind::ALL
        .into_iter()
        .filter(|kind| weights.get(&kind.raw()).copied().unwrap_or(0.0) > 0.0)
        .collect::<Vec<_>>();
    if enabled.len() as u32 > block_count {
        return Err(compile_error());
    }
    let mut blocks = BTreeMap::new();
    let mut remainders = Vec::new();
    let mut used = 0_u32;
    for kind in &enabled {
        let exact = weights[&kind.raw()] / sum * f64::from(block_count);
        let allocated = (exact.floor() as u32).max(1);
        blocks.insert(kind.raw(), allocated);
        used = used.checked_add(allocated).ok_or_else(compile_error)?;
        remainders.push((exact - exact.floor(), kind.raw()));
    }
    if used > block_count {
        return Err(compile_error());
    }
    remainders.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    for index in 0..(block_count - used) {
        let key = remainders[index as usize % remainders.len()].1;
        *blocks.get_mut(&key).expect("enabled lobe block") += 1;
    }
    let mut regions = Vec::with_capacity(LobeKind::ALL.len());
    let mut cursor = 0_u32;
    for kind in LobeKind::ALL {
        let count = blocks.get(&kind.raw()).copied().unwrap_or(0);
        if count == 0 {
            regions.push(LobeRegion::disabled(kind, cursor));
        } else {
            let len = count * 16;
            regions.push(LobeRegion::enabled(kind, cursor, len));
            cursor += len;
        }
    }
    let layout = LobeLayout { regions };
    layout.validate_for_neuron_count(neurons)?;
    Ok(layout)
}

const fn compile_error() -> ScaffoldContractError {
    ScaffoldContractError::PhenotypeCompile
}
