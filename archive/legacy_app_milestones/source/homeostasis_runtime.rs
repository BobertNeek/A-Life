//! CA15 bounded endocrine/homeostasis runtime presentation.
//!
//! This app-layer module turns the core `HomeostaticSnapshot` into fixed-size,
//! player-facing registers. It is display and telemetry only; the source of
//! truth remains `CreatureMind` and the existing core chemistry contract.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeostasisRegisterKind {
    Energy,
    Hunger,
    Fatigue,
    Pain,
    Stress,
}

impl HomeostasisRegisterKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Energy => "Energy",
            Self::Hunger => "Hunger",
            Self::Fatigue => "Fatigue",
            Self::Pain => "Pain",
            Self::Stress => "Stress",
        }
    }

    const fn all() -> [Self; CA15_HOMEOSTASIS_REGISTER_COUNT] {
        [
            Self::Energy,
            Self::Hunger,
            Self::Fatigue,
            Self::Pain,
            Self::Stress,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeostasisRegisterBand {
    Low,
    Normal,
    High,
}

impl HomeostasisRegisterBand {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomeostasisRegisterPresentation {
    pub kind: HomeostasisRegisterKind,
    pub value: f32,
    pub band: HomeostasisRegisterBand,
}

impl HomeostasisRegisterPresentation {
    pub fn new(kind: HomeostasisRegisterKind, value: f32) -> Result<Self, ScaffoldContractError> {
        let value = NormalizedScalar::new(value)?.raw();
        Ok(Self {
            kind,
            value,
            band: register_band(kind, value),
        })
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        NormalizedScalar::new(self.value)?;
        Ok(())
    }

    pub fn bar_line(&self) -> String {
        format!(
            "{:<7} [{}] {:>3}% {}",
            self.kind.label(),
            homeostasis_bar(self.value),
            (self.value * 100.0).round() as u32,
            self.band.label()
        )
    }

    pub fn compact_pair(&self) -> String {
        let label = match self.kind {
            HomeostasisRegisterKind::Energy => "E",
            HomeostasisRegisterKind::Hunger => "H",
            HomeostasisRegisterKind::Fatigue => "F",
            HomeostasisRegisterKind::Pain => "P",
            HomeostasisRegisterKind::Stress => "S",
        };
        format!("{label}{:.2}", self.value)
    }

    fn signature_line(self) -> String {
        format!("{:?}:{:.3}:{}", self.kind, self.value, self.band.label())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HomeostasisRuntimePresentation {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub registers: [HomeostasisRegisterPresentation; CA15_HOMEOSTASIS_REGISTER_COUNT],
    pub salience_modulation: f32,
    pub learning_modulation: f32,
    pub threshold_scale: f32,
    pub motor_confidence_scale: f32,
    pub source: &'static str,
    pub fixed_register_count: bool,
    pub no_heap_growth_hot_path: bool,
}

impl HomeostasisRuntimePresentation {
    pub fn pending(organism_id: OrganismId, tick: Tick) -> Self {
        Self {
            schema: CA15_HOMEOSTASIS_RUNTIME_SCHEMA,
            schema_version: CA15_HOMEOSTASIS_RUNTIME_SCHEMA_VERSION,
            organism_id,
            tick,
            registers: [
                HomeostasisRegisterPresentation {
                    kind: HomeostasisRegisterKind::Energy,
                    value: 0.0,
                    band: HomeostasisRegisterBand::Low,
                },
                HomeostasisRegisterPresentation {
                    kind: HomeostasisRegisterKind::Hunger,
                    value: 0.0,
                    band: HomeostasisRegisterBand::Low,
                },
                HomeostasisRegisterPresentation {
                    kind: HomeostasisRegisterKind::Fatigue,
                    value: 0.0,
                    band: HomeostasisRegisterBand::Low,
                },
                HomeostasisRegisterPresentation {
                    kind: HomeostasisRegisterKind::Pain,
                    value: 0.0,
                    band: HomeostasisRegisterBand::Low,
                },
                HomeostasisRegisterPresentation {
                    kind: HomeostasisRegisterKind::Stress,
                    value: 0.0,
                    band: HomeostasisRegisterBand::Low,
                },
            ],
            salience_modulation: 0.0,
            learning_modulation: 0.0,
            threshold_scale: 0.0,
            motor_confidence_scale: 0.0,
            source: "pending",
            fixed_register_count: true,
            no_heap_growth_hot_path: true,
        }
    }

    pub fn from_live_loop(live: &LiveBrainLoop) -> Result<Self, GameAppShellError> {
        Self::from_snapshot(live.organism_id(), live.mind().homeostasis())
    }

    pub fn from_snapshot(
        organism_id: OrganismId,
        snapshot: &HomeostaticSnapshot,
    ) -> Result<Self, GameAppShellError> {
        snapshot.validate_contract()?;
        let parameters = HomeostaticParameters::reference();
        let stress = snapshot
            .hormones
            .cortisol
            .max(snapshot.hormones.adrenaline)
            .max(snapshot.drives.fear)
            .max(snapshot.drives.temperature_stress);
        let registers = [
            HomeostasisRegisterPresentation::new(
                HomeostasisRegisterKind::Energy,
                snapshot.drives.brain_atp,
            )?,
            HomeostasisRegisterPresentation::new(
                HomeostasisRegisterKind::Hunger,
                snapshot.drives.hunger,
            )?,
            HomeostasisRegisterPresentation::new(
                HomeostasisRegisterKind::Fatigue,
                snapshot.drives.fatigue,
            )?,
            HomeostasisRegisterPresentation::new(
                HomeostasisRegisterKind::Pain,
                snapshot.drives.pain,
            )?,
            HomeostasisRegisterPresentation::new(HomeostasisRegisterKind::Stress, stress)?,
        ];
        let presentation = Self {
            schema: CA15_HOMEOSTASIS_RUNTIME_SCHEMA,
            schema_version: CA15_HOMEOSTASIS_RUNTIME_SCHEMA_VERSION,
            organism_id,
            tick: snapshot.tick,
            registers,
            salience_modulation: ChemistryModulation::salience_weight(snapshot, parameters)?,
            learning_modulation: ChemistryModulation::learning_rate_scale(snapshot, parameters)?,
            threshold_scale: ChemistryModulation::threshold_scale(snapshot, parameters)?,
            motor_confidence_scale: ChemistryModulation::motor_confidence(
                Confidence::new(1.0)?,
                snapshot,
                parameters,
            )?
            .raw(),
            source: "alife_core.HomeostaticSnapshot",
            fixed_register_count: true,
            no_heap_growth_hot_path: true,
        };
        presentation.validate()?;
        Ok(presentation)
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA15_HOMEOSTASIS_RUNTIME_SCHEMA
            || self.schema_version != CA15_HOMEOSTASIS_RUNTIME_SCHEMA_VERSION
            || !self.fixed_register_count
            || !self.no_heap_growth_hot_path
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA15 homeostasis presentation must be fixed-size and current",
            });
        }
        self.organism_id.validate()?;
        for expected in HomeostasisRegisterKind::all() {
            if self
                .registers
                .iter()
                .filter(|register| register.kind == expected)
                .count()
                != 1
            {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA15 homeostasis presentation must contain each register once",
                });
            }
        }
        for register in &self.registers {
            register.validate()?;
        }
        for value in [
            self.salience_modulation,
            self.learning_modulation,
            self.threshold_scale,
            self.motor_confidence_scale,
        ] {
            NormalizedScalar::new(value)?;
        }
        if self.panel_text().contains("Entity(") {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA15 homeostasis presentation must not expose Bevy Entity IDs",
            });
        }
        Ok(())
    }

    pub fn register_value(&self, kind: HomeostasisRegisterKind) -> f32 {
        self.registers
            .iter()
            .find(|register| register.kind == kind)
            .map_or(0.0, |register| register.value)
    }

    pub fn compact_line(&self) -> String {
        format!(
            "Homeo {} {} {} {} {}",
            self.registers[0].compact_pair(),
            self.registers[1].compact_pair(),
            self.registers[2].compact_pair(),
            self.registers[3].compact_pair(),
            self.registers[4].compact_pair()
        )
    }

    pub fn modulation_line(&self) -> String {
        format!(
            "Mods: sal={:.2} learn={:.2}",
            self.salience_modulation, self.learning_modulation
        )
    }

    pub fn panel_text(&self) -> String {
        format!(
            "Homeostasis\n{}\n{}\nBoundary: fixed registers; core-owned snapshot",
            self.registers
                .iter()
                .map(HomeostasisRegisterPresentation::bar_line)
                .collect::<Vec<_>>()
                .join("\n"),
            self.modulation_line()
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{:.3}:{:.3}:{:.3}:{:.3}:{}",
            self.schema,
            self.schema_version,
            self.organism_id.raw(),
            self.tick.raw(),
            self.salience_modulation,
            self.learning_modulation,
            self.threshold_scale,
            self.motor_confidence_scale,
            self.registers
                .iter()
                .map(|register| register.signature_line())
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HomeostasisRuntimeSmokeSummary {
    pub before: HomeostasisRuntimePresentation,
    pub after: HomeostasisRuntimePresentation,
    pub patch_sealed: bool,
    pub finite_and_bounded: bool,
    pub fixed_register_count: bool,
    pub salience_learning_visible: bool,
}

impl HomeostasisRuntimeSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.before.validate()?;
        self.after.validate()?;
        if !self.patch_sealed
            || !self.finite_and_bounded
            || !self.fixed_register_count
            || !self.salience_learning_visible
            || self.after.registers.len() != CA15_HOMEOSTASIS_REGISTER_COUNT
            || !self.after.panel_text().contains("Homeostasis")
            || !self.after.panel_text().contains("sal=")
            || !self.after.panel_text().contains("learn=")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA15 homeostasis smoke must prove bounded bars and modulation",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}->{}:sealed={}:fixed={}:visible={}",
            self.before.signature_line(),
            self.after.signature_line(),
            self.patch_sealed,
            self.fixed_register_count,
            self.salience_learning_visible
        )
    }
}

pub fn run_homeostasis_runtime_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<HomeostasisRuntimeSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let before = HomeostasisRuntimePresentation::from_live_loop(&live)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let summaries = panel.apply_command(&mut live, RuntimeControlCommand::StepOnce)?;
    let after = panel.homeostasis.clone();
    let patch_sealed = summaries.iter().all(|summary| summary.patch_sealed);
    let finite_and_bounded = after
        .registers
        .iter()
        .all(|register| register.value.is_finite() && (0.0..=1.0).contains(&register.value))
        && [
            after.salience_modulation,
            after.learning_modulation,
            after.threshold_scale,
            after.motor_confidence_scale,
        ]
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value));
    let summary = HomeostasisRuntimeSmokeSummary {
        before,
        after,
        patch_sealed,
        finite_and_bounded,
        fixed_register_count: true,
        salience_learning_visible: true,
    };
    summary.validate()?;
    Ok(summary)
}

fn register_band(kind: HomeostasisRegisterKind, value: f32) -> HomeostasisRegisterBand {
    match kind {
        HomeostasisRegisterKind::Energy if value < 0.25 => HomeostasisRegisterBand::Low,
        HomeostasisRegisterKind::Energy => HomeostasisRegisterBand::Normal,
        _ if value >= 0.72 => HomeostasisRegisterBand::High,
        _ if value <= 0.15 => HomeostasisRegisterBand::Low,
        _ => HomeostasisRegisterBand::Normal,
    }
}

fn homeostasis_bar(value: f32) -> String {
    let filled = (value.clamp(0.0, 1.0) * 8.0).round() as usize;
    format!(
        "{}{}",
        "#".repeat(filled),
        ".".repeat(8usize.saturating_sub(filled))
    )
}
