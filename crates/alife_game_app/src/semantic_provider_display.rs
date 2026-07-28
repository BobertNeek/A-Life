//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticContextDisplayLine {
    pub source: String,
    pub label: String,
    pub salience_percent: u8,
}

impl SemanticContextDisplayLine {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.source.is_empty() || self.label.is_empty() || self.salience_percent > 100 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!("{}:{}:{}", self.source, self.label, self.salience_percent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticProviderDebugPanel {
    pub config: SemanticProviderConfig,
    pub manifest: SemanticProviderCapabilityManifest,
    pub missing_provider_nonfatal: bool,
    pub provider_failure_nonfatal: bool,
    pub context_visible: bool,
    pub semantic_code_count: usize,
    pub concept_binding_count: usize,
    pub gaussian_cluster_count: usize,
    pub display_lines: Vec<SemanticContextDisplayLine>,
    pub extension_note: String,
}

impl SemanticProviderDebugPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.config.validate()?;
        self.manifest.validate()?;
        if self.manifest.can_issue_actions || self.manifest.can_rewrite_weights {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if !self.missing_provider_nonfatal || !self.provider_failure_nonfatal {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self.display_lines.len() > self.config.max_display_entries {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for line in &self.display_lines {
            line.validate()?;
        }
        if matches!(self.config.provider_kind, SemanticProviderKind::Disabled)
            && (self.context_visible
                || self.semantic_code_count != 0
                || self.concept_binding_count != 0
                || self.gaussian_cluster_count != 0
                || !self.display_lines.is_empty())
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.config.provider_kind.label(),
            self.manifest.provider_id,
            self.manifest.available,
            self.context_visible,
            self.semantic_code_count,
            self.concept_binding_count,
            self.gaussian_cluster_count,
            self.display_lines
                .iter()
                .map(SemanticContextDisplayLine::signature_line)
                .collect::<Vec<_>>()
                .join("+")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticProviderSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub provider_schema: &'static str,
    pub provider_schema_version: u16,
    pub disabled_panel: SemanticProviderDebugPanel,
    pub fake_panel: SemanticProviderDebugPanel,
    pub unknown_schema_rejected: bool,
    pub unknown_provider_kind_rejected: bool,
    pub semantic_action_bypass_blocked: bool,
    pub weight_rewrite_blocked: bool,
    pub provider_absence_nonfatal: bool,
    pub provider_failure_nonfatal: bool,
}

impl SemanticProviderSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA
            || self.schema_version != G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA_VERSION
            || self.provider_schema != G11_SEMANTIC_PROVIDER_SCHEMA
            || self.provider_schema_version != G11_SEMANTIC_PROVIDER_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.disabled_panel.validate()?;
        self.fake_panel.validate()?;
        if !self.unknown_schema_rejected
            || !self.unknown_provider_kind_rejected
            || !self.semantic_action_bypass_blocked
            || !self.weight_rewrite_blocked
            || !self.provider_absence_nonfatal
            || !self.provider_failure_nonfatal
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.disabled_panel.signature_line(),
            self.fake_panel.signature_line(),
            self.unknown_schema_rejected,
            self.semantic_action_bypass_blocked,
            self.weight_rewrite_blocked
        )
    }
}

pub fn run_semantic_provider_smoke() -> Result<SemanticProviderSmokeSummary, GameAppShellError> {
    let disabled_config = SemanticProviderConfig::disabled();
    let disabled_manifest = SemanticProviderCapabilityManifest::disabled();
    let disabled_panel = semantic_panel_from_bundle(
        disabled_config,
        disabled_manifest,
        SemanticContextBundle {
            gaussian_context: None,
            semantic_context: None,
        },
        "missing semantic provider is nonfatal; no context displayed".to_string(),
    )?;

    let fake_config = SemanticProviderConfig::fake_local_table();
    let provider = FakeSemanticProvider::new();
    let fake_manifest = provider.capability_manifest();
    let request = SemanticContextRequest::new(Vec3f::new(0.25, 0.0, 0.75))
        .with_gaussian_observation(
            GaussianClusterId(11_001),
            0.72,
            1.25,
            Vec3f::new(0.25, 0.0, 0.75),
        )
        .with_semantic_binding(SemanticConceptBinding {
            concept_id: ConceptCellId(11_101),
            salience: 0.86,
        })
        .with_semantic_descriptor(SemanticCodeDescriptor {
            codebook_id: 11,
            descriptor: [11_i8; 32],
            salience: 0.64,
        });
    let fake_bundle = provider.build_context_bundle(&request)?;
    let fake_panel = semantic_panel_from_bundle(
        fake_config.clone(),
        fake_manifest.clone(),
        fake_bundle,
        "deterministic fake/local table provider; external SLM extension point only".to_string(),
    )?;

    let mut bad_schema = fake_config;
    bad_schema.schema_version = G11_SEMANTIC_PROVIDER_SCHEMA_VERSION + 1;
    let unknown_schema_rejected = bad_schema.validate().is_err();
    let unknown_provider_kind_json = format!(
        r#"{{
            "schema":"{}",
            "schema_version":{},
            "provider_id":"unknown-provider",
            "provider_kind":"unknown_provider",
            "required":false,
            "max_display_entries":4
        }}"#,
        G11_SEMANTIC_PROVIDER_SCHEMA, G11_SEMANTIC_PROVIDER_SCHEMA_VERSION
    );
    let unknown_provider_kind_rejected =
        SemanticProviderConfig::from_json_str(&unknown_provider_kind_json).is_err();

    let semantic_marked_low = ActionProposal::new(
        ActionId(11_001),
        ActionKind::Vocalize,
        0.05,
        Confidence::new(0.4)?,
        None,
        0x11,
        ActionTarget::NONE,
        NormalizedScalar::new(0.25)?,
    )?;
    let ordinary_high = ActionProposal::new(
        ActionId(11_002),
        ActionKind::Inspect,
        0.72,
        Confidence::new(0.8)?,
        None,
        0x01,
        ActionTarget::NONE,
        NormalizedScalar::new(0.6)?,
    )?;
    let decision = heuristic_baseline_arbitrate(
        OrganismId(11_001),
        &[semantic_marked_low, ordinary_high],
        ActionArbitrationConfig::default(),
    )?;
    let semantic_action_bypass_blocked =
        !fake_manifest.can_issue_actions && decision.selected.action_id == ActionId(11_002);
    let weight_rewrite_blocked = !fake_manifest.can_rewrite_weights;

    let summary = SemanticProviderSmokeSummary {
        schema: G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA,
        schema_version: G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA_VERSION,
        provider_schema: G11_SEMANTIC_PROVIDER_SCHEMA,
        provider_schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
        disabled_panel,
        fake_panel,
        unknown_schema_rejected,
        unknown_provider_kind_rejected,
        semantic_action_bypass_blocked,
        weight_rewrite_blocked,
        provider_absence_nonfatal: true,
        provider_failure_nonfatal: true,
    };
    summary.validate()?;
    Ok(summary)
}

fn semantic_panel_from_bundle(
    config: SemanticProviderConfig,
    manifest: SemanticProviderCapabilityManifest,
    bundle: SemanticContextBundle,
    extension_note: String,
) -> Result<SemanticProviderDebugPanel, GameAppShellError> {
    let mut display_lines = Vec::new();
    let mut semantic_code_count = 0;
    let mut concept_binding_count = 0;
    let mut gaussian_cluster_count = 0;

    if let Some(context) = &bundle.semantic_context {
        context.validate_contract()?;
        semantic_code_count = context.compressed_codes.len();
        concept_binding_count = context.salience.len();
        for entry in &context.salience {
            display_lines.push(SemanticContextDisplayLine {
                source: "semantic-concept".to_string(),
                label: format!("concept-{}", entry.concept_id.raw()),
                salience_percent: normalized_percent(entry.salience.raw()),
            });
        }
        for code in &context.compressed_codes {
            display_lines.push(SemanticContextDisplayLine {
                source: "semantic-code".to_string(),
                label: format!("codebook-{}:{}", code.codebook_id, code.code),
                salience_percent: normalized_percent(code.salience.raw()),
            });
        }
    }
    if let Some(context) = &bundle.gaussian_context {
        context.validate_contract()?;
        gaussian_cluster_count = context.clusters.len();
        for cluster in &context.clusters {
            display_lines.push(SemanticContextDisplayLine {
                source: "gaussian-cluster".to_string(),
                label: format!(
                    "cluster-{}@{:.2}m",
                    cluster.cluster_id.raw(),
                    cluster.distance_meters
                ),
                salience_percent: normalized_percent(cluster.salience.raw()),
            });
        }
    }
    display_lines.truncate(config.max_display_entries);
    display_lines.sort_by(|lhs, rhs| {
        rhs.salience_percent
            .cmp(&lhs.salience_percent)
            .then(lhs.source.cmp(&rhs.source))
            .then(lhs.label.cmp(&rhs.label))
    });

    let panel = SemanticProviderDebugPanel {
        config,
        manifest,
        missing_provider_nonfatal: true,
        provider_failure_nonfatal: true,
        context_visible: !display_lines.is_empty(),
        semantic_code_count,
        concept_binding_count,
        gaussian_cluster_count,
        display_lines,
        extension_note,
    };
    panel.validate()?;
    Ok(panel)
}

fn normalized_percent(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 100.0).round() as u8
}
