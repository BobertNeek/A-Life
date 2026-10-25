//! Optional host-side observation of ordinary production ticks. Never replay input.
use super::*;
use std::io::Write;

#[cfg(feature = "bevy-app")]
const DEFAULT_MAX_TICKS: u32 = 10_000;

/// A streaming sink: reaching its bound or losing output never stops the organism.
pub(crate) struct HeadlessActionTrace {
    path: PathBuf,
    file: Option<std::fs::File>,
    max_ticks: u32,
    written_ticks: u32,
    first_world_tick: Option<u64>,
    last_world_tick: Option<u64>,
    error: Option<String>,
}

impl HeadlessActionTrace {
    #[cfg(feature = "bevy-app")]
    pub(crate) fn from_environment() -> Option<Self> {
        let path = std::env::var_os("ALIFE_ACTION_TRACE_PATH")?;
        let limit = std::env::var_os("ALIFE_HEADLESS_TRACE_MAX_TICKS")
            .map(|value| value.to_string_lossy().parse::<u32>())
            .transpose();
        Some(Self::open(
            path.into(),
            match limit {
                Ok(limit) => Ok(limit.unwrap_or(DEFAULT_MAX_TICKS)),
                Err(error) => Err(error.to_string()),
            },
        ))
    }

    fn open(path: PathBuf, limit: std::result::Result<u32, String>) -> Self {
        let mut trace = Self {
            path,
            file: None,
            max_ticks: 0,
            written_ticks: 0,
            first_world_tick: None,
            last_world_tick: None,
            error: None,
        };
        match limit {
            Ok(limit) if (1..=1_000_000).contains(&limit) => {
                trace.max_ticks = limit;
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&trace.path)
                {
                    Ok(file) => trace.file = Some(file),
                    Err(error) => trace.fail(error.to_string()),
                }
            }
            _ => trace.fail("ALIFE_HEADLESS_TRACE_MAX_TICKS must be in 1..=1000000".into()),
        }
        trace
    }

    fn fail(&mut self, error: String) {
        eprintln!("Headless action trace stopped: {error}");
        self.error = Some(error);
        self.file = None;
    }

    pub(crate) fn record(&mut self, tick: Tick, row: impl FnOnce() -> serde_json::Value) {
        if self.file.is_none() || self.written_ticks >= self.max_ticks {
            return;
        }
        let row = row();
        if let Err(error) = writeln!(self.file.as_mut().expect("open trace"), "{row}") {
            self.fail(error.to_string());
            return;
        }
        self.written_ticks += 1;
        self.first_world_tick.get_or_insert(tick.raw());
        self.last_world_tick = Some(tick.raw());
    }

    pub(crate) fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "scope": "headless_authoritative_ticks",
            "max_ticks": self.max_ticks,
            "written_ticks": self.written_ticks,
            "first_world_tick": self.first_world_tick,
            "last_world_tick": self.last_world_tick,
            "limit_reached": self.max_ticks > 0 && self.written_ticks >= self.max_ticks,
            "error": self.error,
        })
    }
}

#[cfg(feature = "bevy-app")]
impl GpuLiveBrainRuntime {
    pub(crate) fn headless_action_trace_row(
        &self,
        summaries: &[LiveBrainTickSummary],
        elapsed_seconds: f64,
    ) -> serde_json::Value {
        trace_row(
            &self.world,
            summaries,
            &self.last_sealed_patches,
            |id| {
                self.residents
                    .get(&id.raw())
                    .map(|r| r.sleep_scheduler.state())
            },
            elapsed_seconds,
        )
    }
}

/// All inputs are existing CPU-side projections. No checkpoint or neural readback.
pub(crate) fn trace_row(
    world: &HeadlessWorld,
    summaries: &[LiveBrainTickSummary],
    patches: &[ExperiencePatch],
    sleep_state: impl Fn(OrganismId) -> Option<SleepState>,
    elapsed_seconds: f64,
) -> serde_json::Value {
    let actions = summaries
        .iter()
        .map(|summary| {
            // Latest patches survive sleep and failed/preparation ticks. Only a sealed
            // decision from this very batch may supply action-specific consequences.
            let patch = patches.iter().find(|patch| matching_patch(summary, patch));
            serde_json::json!({
                "organism_id": summary.organism_id.raw(),
                "tick_before": summary.world_tick_before.raw(),
                "tick_after": summary.world_tick_after.raw(),
                "status": format!("{:?}", summary.status),
                "action_scope": "representative_candidate",
                "action": summary.selected_action_kind,
                "action_id": summary.selected_action_id,
                "target_id": summary.target_entity,
                "sequence_id": summary.patch_sequence_id,
                "sealed": summary.patch_sealed,
                "learning_updates": summary.learning_updates,
                "selected_bundle": patch.and_then(|patch| patch.selected_bundle()),
                "selected_action": patch.map(|patch| &patch.decision().selected_action),
                // Includes per-channel physical outcomes and measured before/after
                // physiology. Consumption and nourishment remain distinct evidence.
                "outcome": patch.map(|patch| patch.outcome()),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "scope": "headless_authoritative_tick",
        "world_tick": world.tick().raw(),
        "elapsed_seconds": elapsed_seconds,
        "actions": actions,
        "organisms": world.organism_registry().iter().map(|record| {
            let biology = record.biochemistry();
            serde_json::json!({
                "organism_id": record.organism_id().raw(),
                "stable_id": record.world_entity_id().raw(),
                "neural_sleep": sleep_state(record.organism_id()),
                "sleep_phase": record.sleep_phase(),
                "sleep_cycle_id": record.sleep_cycle_id(),
                "body_sleeping": biology.body.sleeping,
                "energy": biology.body.energy,
                "health": biology.body.health,
                "hunger": biology.homeostasis.drives.hunger,
                "fatigue": biology.homeostasis.drives.fatigue,
                "brain_atp": biology.homeostasis.drives.brain_atp,
                "death_tick": record.lifecycle().death_tick(),
            })
        }).collect::<Vec<_>>(),
        "food": world.object_snapshots().iter()
            .filter(|object| object.kind == WorldObjectKind::Food)
            .map(|object| serde_json::json!({
                "id": object.id.raw(),
                "position": object.position,
                "consumed": object.consumed,
                "carried_by": object.carried_by,
            })).collect::<Vec<_>>(),
    })
}

fn matching_patch(summary: &LiveBrainTickSummary, patch: &ExperiencePatch) -> bool {
    summary.patch_sealed
        && patch.header().phase == alife_core::ExperiencePatchPhase::Sealed
        && summary.organism_id == patch.header().organism_id
        && summary.patch_sequence_id == Some(patch.header().sequence_id.raw())
        && summary.world_tick_before == patch.header().world_tick
        && summary.world_tick_after == patch.outcome().outcome_tick
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "alife-headless-trace-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&root).unwrap();
        root.join("trace.jsonl")
    }

    #[test]
    fn bounded_stream_reports_coverage_and_never_overwrites_evidence() {
        let path = path();
        let mut trace = HeadlessActionTrace::open(path.clone(), Ok(2));
        for raw in 21..=22 {
            trace.record(Tick(raw), || serde_json::json!({"world_tick": raw}));
        }
        trace.record(Tick(23), || {
            panic!("a capped trace must do no snapshot work")
        });
        assert_eq!(trace.status()["written_ticks"], 2);
        assert_eq!(trace.status()["first_world_tick"], 21);
        assert_eq!(trace.status()["last_world_tick"], 22);
        assert_eq!(trace.status()["limit_reached"], true);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(String::from_utf8(bytes.clone()).unwrap().lines().count(), 2);
        let mut refused = HeadlessActionTrace::open(path.clone(), Ok(2));
        refused.record(Tick(24), || {
            panic!("refused output must disable observation")
        });
        assert!(refused.status()["error"].is_string());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        drop(trace);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn invalid_horizon_does_not_create_output_and_empty_world_has_no_substitute() {
        let path = path();
        let trace = HeadlessActionTrace::open(path.clone(), Ok(1_000_001));
        assert!(trace.status()["error"].is_string());
        assert!(!path.exists());
        let world = alife_world::HeadlessScenarioBuilder::new(31)
            .build()
            .unwrap();
        let row = trace_row(&world, &[], &[], |_| None, 0.0);
        assert_eq!(row["organisms"], serde_json::json!([]));
        assert_eq!(row["actions"], serde_json::json!([]));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
