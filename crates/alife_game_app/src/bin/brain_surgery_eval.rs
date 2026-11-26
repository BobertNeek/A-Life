//! Headless CLI for self-contained brain surgery bundles, data-only patches,
//! separate-process V11 reopen checks, and matched scorecard comparisons.

use std::path::PathBuf;

use alife_game_app::brain_surgery::{
    apply_patch_to_bundle, compare_bundles, init_base_bundle, init_base_bundle_from_save,
    load_and_verify_base_bundle, validate_and_apply_patch, verify_bundle_reopen,
    verify_bundle_reopen_child, BrainSurgeryPatchV1, Result,
};

fn usage() -> &'static str {
    "usage:
  brain_surgery_eval init-base --output DIR --base-id ID [--founder-seed N] [--foundation PATH] [--branches PATH] [--from-save SAVE --asset-root DIR [--organism-id ID]] [--scorecard PATH]
  brain_surgery_eval validate-patch --base DIR --patch PATH [--candidate-id ID]
  brain_surgery_eval apply-patch --base DIR --patch PATH --out DIR [--candidate-id ID]
  brain_surgery_eval verify-reopen --bundle DIR [--seed N] [--stamp-bundle] [--out RECEIPT.json]
  brain_surgery_eval verify-reopen-child SAVE ASSET_ROOT CONFIG ASSETS EXPECTED
  brain_surgery_eval score --base DIR --candidate DIR --out REPORT.json [--scorecard PATH] [--patch-id ID] [--min-improvement F] [--work-dir DIR]"
}

fn next_flag_value(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    let next = index.checked_add(1).ok_or("argument index overflow")?;
    let value = args
        .get(next)
        .ok_or_else(|| format!("missing value for {flag}"))?
        .clone();
    *index = next + 1;
    Ok(value)
}

fn cmd_init_base(args: &[String]) -> Result<()> {
    let mut output: Option<PathBuf> = None;
    let mut base_id: Option<String> = None;
    let mut founder_seed: u64 = 700_001;
    let mut foundation: Option<PathBuf> = None;
    let mut branches: Option<PathBuf> = None;
    let mut from_save: Option<PathBuf> = None;
    let mut asset_root: Option<PathBuf> = None;
    let mut organism_id: Option<u64> = None;
    let mut scorecard: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => output = Some(PathBuf::from(next_flag_value(args, &mut i, "--output")?)),
            "--base-id" => base_id = Some(next_flag_value(args, &mut i, "--base-id")?),
            "--founder-seed" => {
                founder_seed = next_flag_value(args, &mut i, "--founder-seed")?.parse()?
            }
            "--foundation" => {
                foundation = Some(PathBuf::from(next_flag_value(
                    args,
                    &mut i,
                    "--foundation",
                )?))
            }
            "--branches" => {
                branches = Some(PathBuf::from(next_flag_value(args, &mut i, "--branches")?))
            }
            "--from-save" => {
                from_save = Some(PathBuf::from(next_flag_value(args, &mut i, "--from-save")?))
            }
            "--asset-root" => {
                asset_root = Some(PathBuf::from(next_flag_value(
                    args,
                    &mut i,
                    "--asset-root",
                )?))
            }
            "--organism-id" => {
                organism_id = Some(next_flag_value(args, &mut i, "--organism-id")?.parse()?)
            }
            "--scorecard" => {
                scorecard = Some(PathBuf::from(next_flag_value(args, &mut i, "--scorecard")?))
            }
            other => return Err(format!("unknown flag {other} for init-base\n{}", usage()).into()),
        }
    }

    let output = output.ok_or("init-base requires --output")?;
    let base_id = base_id.ok_or("init-base requires --base-id")?;
    let bundle = match from_save {
        Some(save_path) => {
            if foundation.is_some() || branches.is_some() {
                return Err(
                    "--from-save cannot be combined with --foundation or --branches".into(),
                );
            }
            let asset_root = asset_root.ok_or("--from-save requires --asset-root")?;
            init_base_bundle_from_save(
                &output,
                &base_id,
                &save_path,
                &asset_root,
                organism_id,
                scorecard.as_deref(),
            )?
        }
        None => init_base_bundle(
            &output,
            &base_id,
            foundation.as_deref(),
            branches.as_deref(),
            scorecard.as_deref(),
            founder_seed,
        )?,
    };
    println!("{}", serde_json::to_string_pretty(&bundle.manifest)?);
    Ok(())
}

fn cmd_validate_patch(args: &[String]) -> Result<()> {
    let mut base: Option<PathBuf> = None;
    let mut patch_path: Option<PathBuf> = None;
    let mut candidate_id: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--base" => base = Some(PathBuf::from(next_flag_value(args, &mut i, "--base")?)),
            "--patch" => {
                patch_path = Some(PathBuf::from(next_flag_value(args, &mut i, "--patch")?))
            }
            "--candidate-id" => {
                candidate_id = Some(next_flag_value(args, &mut i, "--candidate-id")?)
            }
            other => {
                return Err(format!("unknown flag {other} for validate-patch\n{}", usage()).into())
            }
        }
    }
    let base = load_and_verify_base_bundle(&base.ok_or("validate-patch requires --base")?)?;
    let patch: BrainSurgeryPatchV1 =
        serde_json::from_slice(&std::fs::read(patch_path.ok_or("requires --patch")?)?)?;
    let id = candidate_id.unwrap_or_else(|| patch.patch_id.clone());
    let (_, _, receipt) = validate_and_apply_patch(&base, &patch, &id)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn cmd_apply_patch(args: &[String]) -> Result<()> {
    let mut base: Option<PathBuf> = None;
    let mut patch: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut candidate_id: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--base" => base = Some(PathBuf::from(next_flag_value(args, &mut i, "--base")?)),
            "--patch" => patch = Some(PathBuf::from(next_flag_value(args, &mut i, "--patch")?)),
            "--out" => out = Some(PathBuf::from(next_flag_value(args, &mut i, "--out")?)),
            "--candidate-id" => {
                candidate_id = Some(next_flag_value(args, &mut i, "--candidate-id")?)
            }
            other => {
                return Err(format!("unknown flag {other} for apply-patch\n{}", usage()).into())
            }
        }
    }
    let receipt = apply_patch_to_bundle(
        &base.ok_or("apply-patch requires --base")?,
        &patch.ok_or("apply-patch requires --patch")?,
        &out.ok_or("apply-patch requires --out")?,
        candidate_id.as_deref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn cmd_verify_reopen(args: &[String]) -> Result<()> {
    let mut bundle: Option<PathBuf> = None;
    let mut seed: u64 = 700_001;
    let mut stamp_bundle = false;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bundle" => bundle = Some(PathBuf::from(next_flag_value(args, &mut i, "--bundle")?)),
            "--seed" => seed = next_flag_value(args, &mut i, "--seed")?.parse()?,
            "--stamp-bundle" => {
                stamp_bundle = true;
                i += 1;
            }
            "--out" => out = Some(PathBuf::from(next_flag_value(args, &mut i, "--out")?)),
            other => {
                return Err(format!("unknown flag {other} for verify-reopen\n{}", usage()).into())
            }
        }
    }
    let receipt = verify_bundle_reopen(
        &bundle.ok_or("verify-reopen requires --bundle")?,
        seed,
        stamp_bundle,
    )?;
    let encoded = serde_json::to_vec_pretty(&receipt)?;
    if let Some(out_path) = out {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        alife_storage::write(&out_path, &encoded)?;
    }
    println!("{}", String::from_utf8(encoded)?);
    Ok(())
}

fn cmd_verify_reopen_child(args: &[String]) -> Result<()> {
    if args.len() != 5 {
        return Err(usage().into());
    }
    let receipt = verify_bundle_reopen_child(
        PathBuf::from(&args[0]).as_path(),
        PathBuf::from(&args[1]).as_path(),
        PathBuf::from(&args[2]).as_path(),
        PathBuf::from(&args[3]).as_path(),
        PathBuf::from(&args[4]).as_path(),
    )?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn cmd_score(args: &[String]) -> Result<()> {
    let mut base: Option<PathBuf> = None;
    let mut candidate: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut scorecard: Option<PathBuf> = None;
    let mut patch_id: Option<String> = None;
    let mut min_improvement: f64 = 0.0;
    let mut work_dir: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--base" => base = Some(PathBuf::from(next_flag_value(args, &mut i, "--base")?)),
            "--candidate" => {
                candidate = Some(PathBuf::from(next_flag_value(args, &mut i, "--candidate")?))
            }
            "--out" => out = Some(PathBuf::from(next_flag_value(args, &mut i, "--out")?)),
            "--scorecard" => {
                scorecard = Some(PathBuf::from(next_flag_value(args, &mut i, "--scorecard")?))
            }
            "--patch-id" => patch_id = Some(next_flag_value(args, &mut i, "--patch-id")?),
            "--min-improvement" => {
                min_improvement = next_flag_value(args, &mut i, "--min-improvement")?.parse()?
            }
            "--work-dir" => {
                work_dir = Some(PathBuf::from(next_flag_value(args, &mut i, "--work-dir")?))
            }
            other => return Err(format!("unknown flag {other} for score\n{}", usage()).into()),
        }
    }

    let base_dir = base.ok_or("score requires --base")?;
    let cand_dir = candidate.ok_or("score requires --candidate")?;
    let out_path = out.ok_or("score requires --out")?;
    let owned_work = work_dir.unwrap_or_else(|| {
        std::env::temp_dir().join(format!("alife-surgery-score-{}", std::process::id()))
    });
    let report = compare_bundles(
        &base_dir,
        &cand_dir,
        scorecard.as_deref(),
        patch_id,
        min_improvement,
        &owned_work,
    )?;
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    alife_storage::write(&out_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() -> Result<()> {
    alife_game_app::init_headless_tracing();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((sub, rest)) = args.split_first() else {
        return Err(usage().into());
    };
    let sub = sub.clone();
    let rest = rest.to_vec();
    let handle = std::thread::Builder::new()
        .name("surgery-worker".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || -> std::result::Result<(), String> {
            match sub.as_str() {
                "init-base" => cmd_init_base(&rest),
                "validate-patch" => cmd_validate_patch(&rest),
                "apply-patch" => cmd_apply_patch(&rest),
                "verify-reopen" => cmd_verify_reopen(&rest),
                "verify-reopen-child" => cmd_verify_reopen_child(&rest),
                "score" => cmd_score(&rest),
                _ => Err(usage().into()),
            }
            .map_err(|err| err.to_string())
        })?;
    handle
        .join()
        .map_err(|_| "worker panicked")?
        .map_err(|err| err.into())
}
