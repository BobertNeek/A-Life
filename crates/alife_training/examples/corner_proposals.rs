use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, FoundationTrainerCheckpoint, TrainingSequence};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    asset: PathBuf,
    genome: PathBuf,
    actor_checkpoint: PathBuf,
    founder_checkpoint: PathBuf,
    outdir: PathBuf,
}
fn bytes<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(v)?)
}
fn load(
    r: &Request,
) -> Result<(
    CpuCanonicalHost,
    CpuFoundationTrainer,
    FoundationWeightAsset,
)> {
    let a = FoundationWeightAsset::decode_canonical(&fs::read(&r.asset)?)?;
    let g: CreatureGenome = serde_json::from_slice(&fs::read(&r.genome)?)?;
    if g.n2048_foundation_candidate
        .as_ref()
        .ok_or("asset")?
        .encode_canonical()?
        != a.encode_canonical()?
    {
        return Err("genepair".into());
    }
    let e = g.express()?;
    let d = foundation_construction_development(
        &e.brain_genome,
        &BrainCapacityClass::n2048(),
        &e.development_state_at(Tick::ZERO)?,
    )?;
    let p = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        e.brain_genome,
        d,
        a.clone(),
    )?
    .0;
    let f: FoundationTrainerCheckpoint = serde_json::from_slice(&fs::read(&r.founder_checkpoint)?)?;
    let fb = bytes(&f)?;
    let t = CpuFoundationTrainer::from_checkpoint(p, a.clone(), f)?;
    assert_eq!(fb, bytes(&t.checkpoint())?);
    let cp: CpuCanonicalCheckpoint = serde_json::from_slice(&fs::read(&r.actor_checkpoint)?)?;
    let cb = bytes(&cp)?;
    let h = CpuCanonicalHost::from_checkpoint(cp, &t)?;
    assert_eq!(cb, bytes(&h.checkpoint(&t)?)?);
    let cp = h.checkpoint(&t)?;
    if cp.schema_version != 18
        || cp.candidate_opportunity_coverage2313 != Some(3)
        || cp.joint_membership2177 != Some(14)
        || h.world().age_death_disabled()
        || h.world().tick().raw() != 626
        || cp.residents.len() != 1
    {
        return Err("exacthealthy626identity".into());
    }
    let rec = h
        .world()
        .organism_registry()
        .iter()
        .next()
        .ok_or("record")?;
    if !rec.lifecycle().is_alive() || bytes(rec.genome())? != bytes(&g)? {
        return Err("actor genome/life".into());
    }
    let mut old = h;
    let before = old.checkpoint(&t)?;
    let oldf = t.checkpoint();
    for (i, w) in t.weights().iter().enumerate() {
        let expected = match i {
            26144 => 0x80000000,
            26146 => 0,
            _ => t.phenotype().synapses()[i].genetic_weight().to_bits(),
        };
        if w.to_bits() != expected {
            return Err("originalg0bits".into());
        }
    }
    let edits = (30048u32..30088).map(|i| (i, 0.4f32)).collect::<Vec<_>>();
    let next = t.calibrated_readout_copy(&edits)?;
    let mut ef = oldf.clone();
    for &(i, w) in &edits {
        ef.weights[i as usize] = w;
    }
    assert_eq!(bytes(&ef)?, bytes(&next.checkpoint())?);
    old.rebase_readout_policy(&t, &next)?;
    let after = old.checkpoint(&next)?;
    let mut ecp = before;
    ecp.founder_weight_digest = after.founder_weight_digest;
    assert_eq!(bytes(&ecp)?, bytes(&after)?);
    Ok((old, next, a))
}
fn replay(t: &CpuFoundationTrainer, row: &CpuCollectedDecision) -> Result<Vec<f32>> {
    let r = t.evaluate_replay(&TrainingSequence {
        phenotype_hash: t.phenotype().phenotype_hash(),
        initial: row.initial.clone(),
        ticks: vec![row.replay_tick.clone()],
        burn_in_ticks: 0,
        memory_candidate_gain: t
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    })?;
    if r.final_activations[0]
        .iter()
        .map(|x| x.to_bits())
        .collect::<Vec<_>>()
        != row
            .final_features
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    {
        return Err("native activations".into());
    }
    Ok(r.candidate_logits[0].clone())
}
fn export(r: &Request) -> Result<()> {
    let (h, t, _) = load(r)?;
    let original = bytes(&h.checkpoint(&t)?)?;
    let founder = bytes(&t.checkpoint())?;
    let mut copy = h.snapshot();
    let step = copy.step(&t, 1.)?;
    let row = step.decisions.first().ok_or("noactualdecision")?;
    let logits = replay(&t, row)?;
    let sampled = sample_cpu_joint_action(
        t.phenotype(),
        row.patch.pre_action().perception(),
        &logits,
        h.checkpoint(&t)?.sampling_seed,
        row.sampling_counter.ok_or("counter")?,
        1.,
    )?;
    assert_eq!(bytes(&sampled)?, bytes(&row.action)?);
    let graph=t.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|{let CompiledSynapseKind::Decoder(c)=s.kind()else{return None};Some(json!({"index":i,"source":s.source(),"head":c.head(),"family":c.family(),"lane":c.input_lane(),"genetic_bits":s.genetic_weight().to_bits(),"policy_bits":t.weights()[i].to_bits(),"policy":t.weights()[i],"activation":row.final_features[s.source()as usize],"offset":row.replay_tick.effective_weight_offsets[i]}))}).collect::<Vec<_>>();
    let families = t
        .phenotype()
        .candidate_decoder()
        .families()
        .iter()
        .map(|f| json!({"family":f.family(),"bias":f.bias()}))
        .collect::<Vec<_>>();
    fs::create_dir(&r.outdir)?;
    fs::create_dir(r.outdir.join("COMMON-BASE"))?;
    write_cpu_private_checkpoint(&r.outdir.join("COMMON-BASE"), &h.checkpoint(&t)?)?;
    let readback = read_cpu_private_checkpoint(&r.outdir.join("COMMON-BASE"))?;
    assert_eq!(bytes(&readback)?, original);
    fs::write(r.outdir.join("COMMON-BASE-FOUNDER.json"), &founder)?;
    fs::write(r.outdir.join("COMMON-BASE-CP.json"), &original)?;
    fs::write(
        r.outdir.join("NATIVE-GRAPH-CONTEXT.json"),
        bytes(
            &json!({"graph":graph,"families":families,"candidates":row.replay_tick.candidates,"logits":logits,"logit_bits":logits.iter().map(|x|x.to_bits()).collect::<Vec<_>>(),"memory_bound":t.phenotype().candidate_decoder().memory_channel().map_or(0.,|m|m.max_candidate_gain()),"cognitive_bound":if t.phenotype().candidate_decoder().memory_channel().is_some_and(|m|m.schema_version()==2){0.5}else{t.phenotype().candidate_decoder().memory_channel().map_or(0.,|m|m.max_candidate_gain())},"actual_action":row.action,"source_sequence":row.patch.pre_action().sequence_id,"original626advanced":false,"qualification_clone_steps":1}),
        )?,
    )?;
    assert_eq!(original, bytes(&h.checkpoint(&t)?)?);
    assert_eq!(founder, bytes(&t.checkpoint())?);
    Ok(())
}
fn qualify(r: &Request, proposals: &str) -> Result<()> {
    let (h, base, asset) = load(r)?;
    let cp = h.checkpoint(&base)?;
    let original = bytes(&cp)?;
    let founder = base.checkpoint();
    let v: Value = serde_json::from_slice(&fs::read(proposals)?)?;
    let mut results = Vec::new();
    for p in v["variants"].as_array().ok_or("variants")? {
        let edits = p["edits"]
            .as_array()
            .ok_or("edits")?
            .iter()
            .map(|e| {
                Ok((
                    u32::try_from(e["index"].as_u64().ok_or("index")?)?,
                    f32::from_bits(u32::try_from(e["new_bits"].as_u64().ok_or("bits")?)?),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        for &(i, _) in &edits {
            let s = base
                .phenotype()
                .synapses()
                .get(i as usize)
                .ok_or("coordinate out of range")?;
            if !matches!(s.kind(),CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::ActionCandidate&&matches!(c.family(),CandidateActionFamily::Approach|CandidateActionFamily::Avoid)&&[0,1,2,16].contains(&c.input_lane()))
            {
                return Err("outofscopeedit".into());
            }
        }
        let next = if edits.is_empty() {
            CpuFoundationTrainer::from_checkpoint(
                base.phenotype().clone(),
                asset.clone(),
                founder.clone(),
            )?
        } else {
            base.calibrated_readout_copy(&edits)?
        };
        let mut f = founder.clone();
        for &(i, w) in &edits {
            f.weights[i as usize] = w;
        }
        assert_eq!(bytes(&f)?, bytes(&next.checkpoint())?);
        let mut warm = h.snapshot();
        if !edits.is_empty() {
            warm.rebase_readout_policy(&base, &next)?;
        }
        let after = warm.checkpoint(&next)?;
        let mut expected = cp.clone();
        expected.founder_weight_digest = after.founder_weight_digest;
        assert_eq!(bytes(&expected)?, bytes(&after)?);
        let mut cold = CpuCanonicalHost::from_checkpoint(after.clone(), &next)?;
        assert_eq!(bytes(&after)?, bytes(&cold.checkpoint(&next)?)?);
        let wa = warm.step(&next, 1.)?;
        let co = cold.step(&next, 1.)?;
        assert_eq!(bytes(&wa.decisions)?, bytes(&co.decisions)?);
        assert_eq!(
            bytes(&warm.checkpoint(&next)?)?,
            bytes(&cold.checkpoint(&next)?)?
        );
        for row in &wa.decisions {
            let logits = replay(&next, row)?;
            let sampled = sample_cpu_joint_action(
                next.phenotype(),
                row.patch.pre_action().perception(),
                &logits,
                cp.sampling_seed,
                row.sampling_counter.ok_or("counter")?,
                1.,
            )?;
            assert_eq!(bytes(&sampled)?, bytes(&row.action)?);
        }
        assert_eq!(bytes(&next.checkpoint())?, bytes(&f)?);
        results.push(json!({"id":p["id"],"edit_count":edits.len(),"founder_other_fields_exact":true,"host_only_digest_change":true,"cold_exact":true,"warm_cold_nextstep_and_native_sampler_exact":true,"changed_weight_indices":edits.iter().map(|(i,_)|*i).collect::<Vec<_>>()}));
    }
    assert_eq!(original, bytes(&h.checkpoint(&base)?)?);
    assert_eq!(bytes(&founder)?, bytes(&base.checkpoint())?);
    write_new(
        &r.outdir.join("NATIVE-VARIANT-QUALIFICATION.json"),
        &bytes(
            &json!({"qualified":true,"variants":results,"original_unchanged":true,"no_actual_tournament_trajectories":true}),
        )?,
    )?;
    Ok(())
}
fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    if a.len() < 3 {
        return Err("usage: export|qualify request.json [proposals.json]".into());
    }
    if a[1] == "qualify" && a.len() != 4 {
        return Err("qualify requires proposals.json".into());
    }
    let r: Request = serde_json::from_slice(&fs::read(&a[2])?)?;
    match a[1].as_str() {
        "export" => export(&r),
        "qualify" => qualify(&r, &a[3]),
        _ => Err("export|qualify manifest [proposals]".into()),
    }
}

fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}
