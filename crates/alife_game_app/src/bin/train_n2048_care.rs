//! Training-only entry point; never linked into the ordinary game executable.

use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mode = args
        .next()
        .ok_or("usage: train_n2048_care --pilot|--teacher-pilot|--cycle OUTPUT_DIRECTORY [TICKS] [--seed N] [--food-after-world-tick N] | --resume-cycle PREVIOUS_DIRECTORY OUTPUT_DIRECTORY [TICKS] [--seed N]")?;
    if mode == "--inspect-cycle" {
        let directory = std::path::PathBuf::from(args.next().ok_or("missing cycle directory")?);
        if args.next().is_some() {
            return Err("unexpected inspect-cycle argument".into());
        }
        return inspect_cycle(&directory);
    }
    if mode != "--pilot"
        && mode != "--teacher-pilot"
        && mode != "--cycle"
        && mode != "--resume-cycle"
    {
        return Err("usage: train_n2048_care --pilot|--teacher-pilot|--cycle OUTPUT_DIRECTORY [TICKS] [--seed N] [--food-after-world-tick N] | --resume-cycle PREVIOUS_DIRECTORY OUTPUT_DIRECTORY [TICKS] [--seed N]".into());
    }
    let first_path = std::path::PathBuf::from(args.next().ok_or("missing output directory")?);
    let (previous, output) = if mode == "--resume-cycle" {
        (
            Some(first_path),
            std::path::PathBuf::from(args.next().ok_or("missing new output directory")?),
        )
    } else {
        (None, first_path)
    };
    let mut ticks = 32;
    let mut next = args.next();
    if let Some(value) = next.as_ref() {
        if !value.to_string_lossy().starts_with("--") {
            ticks = value.to_string_lossy().parse::<usize>()?;
            next = args.next();
        }
    }
    let mut seed = 0x2026_0921;
    let mut food_after_world_tick = None;
    let mut founder_seed_base = None;
    let mut food_position = None;
    while let Some(flag) = next {
        if flag == "--seed" {
            seed = args
                .next()
                .ok_or("missing training seed")?
                .to_string_lossy()
                .parse::<u64>()?;
        } else if flag == "--food-after-world-tick" {
            if food_after_world_tick.is_some() {
                return Err("duplicate food availability flag".into());
            }
            food_after_world_tick = Some(
                args.next()
                    .ok_or("missing food availability tick")?
                    .to_string_lossy()
                    .parse::<u64>()?,
            );
        } else if flag == "--founder-seed-base" {
            if founder_seed_base.is_some() {
                return Err("duplicate founder seed flag".into());
            }
            founder_seed_base = Some(
                args.next()
                    .ok_or("missing founder seed base")?
                    .to_string_lossy()
                    .parse::<u64>()?,
            );
        } else if flag == "--food-position" {
            if food_position.is_some() {
                return Err("duplicate food position flag".into());
            }
            food_position = Some([
                args.next()
                    .ok_or("missing food x")?
                    .to_string_lossy()
                    .parse::<f32>()?,
                args.next()
                    .ok_or("missing food y")?
                    .to_string_lossy()
                    .parse::<f32>()?,
            ]);
        } else {
            return Err("unexpected argument".into());
        }
        next = args.next();
    }
    if mode == "--pilot" || mode == "--teacher-pilot" {
        if food_after_world_tick.is_some() {
            return Err("food availability is only supported in cycle mode".into());
        }
        let receipt = if mode == "--teacher-pilot" {
            alife_game_app::run_foundation_teacher_pilot_with_scenario(
                &output,
                seed,
                founder_seed_base.unwrap_or(seed),
                ticks,
                food_position,
            )?
        } else if founder_seed_base.is_none() && food_position.is_none() {
            alife_game_app::run_foundation_training_pilot(&output, seed, ticks)?
        } else {
            return Err("founder seed and food position overrides need a teacher pilot".into());
        };
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    } else {
        if founder_seed_base.is_some() || food_position.is_some() {
            return Err("founder seed and food position overrides need a teacher pilot".into());
        }
        // The production runtime and replay buffers have large debug-build
        // stack frames on Windows. Keep the CLI's main thread small.
        let receipt = std::thread::Builder::new()
            .name("foundation-training-cycle".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let result = match (previous, food_after_world_tick) {
                    (Some(previous), Some(food_after)) => {
                        alife_game_app::resume_foundation_training_cycle_with_food_delay(
                            &previous, &output, seed, ticks, food_after,
                        )
                    }
                    (Some(previous), None) => alife_game_app::resume_foundation_training_cycle(
                        &previous, &output, seed, ticks,
                    ),
                    (None, Some(food_after)) => {
                        alife_game_app::run_foundation_training_cycle_with_food_delay(
                            &output, seed, ticks, food_after,
                        )
                    }
                    (None, None) => {
                        alife_game_app::run_foundation_training_cycle(&output, seed, ticks)
                    }
                };
                result.map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "training cycle thread panicked")??;
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    }
    Ok(())
}

fn inspect_cycle(directory: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = directory.join("replay-manifest.json");
    let references: Option<Vec<alife_game_app::FoundationReplayRecordRef>> = manifest_path
        .exists()
        .then(|| std::fs::read(&manifest_path))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    let replay_dir = directory.join("replay");
    let files = if let Some(references) = &references {
        references
            .iter()
            .map(|reference| replay_dir.join(reference.file_name()))
            .collect::<Vec<_>>()
    } else {
        let mut files = std::fs::read_dir(&replay_dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("replay-") && name.ends_with(".bin.zst"))
            })
            .collect::<Vec<_>>();
        files.sort();
        files
    };
    if files.is_empty() {
        return Err("cycle has no replay records".into());
    }
    let mut consumed_events = 0_u64;
    let mut first_consumed_tick = None;
    let mut initial_energy = None;
    let mut minimum_waking_energy = f32::INFINITY;
    let mut final_energy = None;
    let mut last_segment = 0_u64;
    let mut last_tick = 0_u64;
    for (index, path) in files.iter().enumerate() {
        let reference = references.as_ref().map(|references| &references[index]);
        let bytes = std::fs::read(path)?;
        if let Some(reference) = reference {
            if reference.record_index != index as u64 {
                return Err("replay manifest index is discontinuous".into());
            }
            if bytes.len() as u64 != reference.encoded_bytes
                || blake3::hash(&bytes).as_bytes() != &reference.digest
            {
                return Err("replay record digest mismatch".into());
            }
        }
        let decoder = zstd::stream::read::Decoder::new(std::io::Cursor::new(bytes))?;
        let record: alife_game_app::FoundationReplayRecord = bincode::serde::decode_from_std_read(
            &mut decoder.take(268_435_457),
            bincode::config::standard().with_limit::<268435456>(),
        )?;
        if record.record_index != index as u64
            || reference.is_some_and(|reference| {
                record.tick != reference.tick || record.segment != reference.segment
            })
            || (index > 0 && record.tick <= last_tick)
        {
            return Err("replay record identity mismatch".into());
        }
        last_tick = record.tick;
        last_segment = record.segment;
        let outcome = record.patch.outcome();
        if outcome.physical.contact == alife_core::PhysicalContactKind::Consumed
            || outcome.joint.as_ref().is_some_and(|joint| {
                joint.channel_outcomes.iter().any(|channel| {
                    channel.physical.contact == alife_core::PhysicalContactKind::Consumed
                })
            })
        {
            consumed_events += 1;
            first_consumed_tick.get_or_insert(record.tick);
        }
        let physiology = outcome
            .measured_physiology
            .ok_or("replay record lacks measured physiology")?;
        initial_energy.get_or_insert(physiology.before.body.energy);
        minimum_waking_energy = minimum_waking_energy
            .min(physiology.before.body.energy)
            .min(physiology.after.body.energy);
        final_energy = Some(physiology.after.body.energy);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "records": files.len(),
            "last_world_tick": last_tick,
            "simulated_minutes": last_tick as f64 / 1200.0,
            "waking_segments": last_segment + 1,
            "consumed_events": consumed_events,
            "first_consumed_tick": first_consumed_tick,
            "initial_energy": initial_energy,
            "minimum_waking_energy": minimum_waking_energy,
            "final_energy": final_energy,
            "cycle_completed": directory.join("cycle.json").exists(),
            "manifest_complete": references.is_some(),
        }))?
    );
    Ok(())
}
