//! Offline readout optimization over validated production GPU observations.

use alife_core::*;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile, GpuSelectorDiagnosticReceipt};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use std::{collections::BTreeSet, sync::mpsc};
use wgpu::util::DeviceExt;

use crate::TrainingError;

/// A pairwise teacher target, separate from the organism's sensory input.
/// Conversion checks receipt structure and graph coordinates. It does not
/// authenticate the origin of a caller-supplied diagnostic receipt.
#[derive(Debug, Clone)]
pub struct ProductionReadoutExample {
    phenotype_hash: PhenotypeHash,
    coefficients: Vec<[f32; 4]>,
    biases: [f32; 2],
}

fn invalid() -> TrainingError {
    ScaffoldContractError::InvalidDecisionEvidence.into()
}

fn readout_ids(phenotype: &BrainPhenotype) -> Vec<usize> {
    phenotype.synapses().iter().enumerate().filter_map(|(i, s)| {
        matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate).then_some(i)
    }).collect()
}

impl ProductionReadoutExample {
    pub fn from_diagnostic(
        phenotype: &BrainPhenotype,
        frame: &PerceptionFrame,
        receipt: &GpuSelectorDiagnosticReceipt,
        preferred: u16,
        rejected: u16,
    ) -> Result<Self, TrainingError> {
        frame.validate_contract()?;
        receipt.validate_contract()?;
        if preferred == rejected
            || receipt.frame_digest != frame.frame_digest()
            || receipt.phenotype_hash != phenotype.phenotype_hash()
        {
            return Err(invalid());
        }
        let ids = readout_ids(phenotype);
        let mut coefficients = vec![[0.0; 4]; ids.len()];
        let mut biases = [0.0; 2];
        for (side, index) in [preferred, rejected].into_iter().enumerate() {
            let candidate = frame
                .candidates()
                .get(usize::from(index))
                .ok_or_else(invalid)?;
            let row = receipt
                .candidates
                .get(usize::from(index))
                .ok_or_else(invalid)?;
            let family = phenotype
                .candidate_decoder()
                .families()
                .iter()
                .find(|f| f.family() == candidate.family)
                .ok_or_else(invalid)?;
            // Constant-only families have no contribution rows to request.
            // The backend still reports their actual bias/final logit.
            if family.decoder_synapse_count() != 0
                && !receipt.requested_candidate_indices.contains(&index)
            {
                return Err(invalid());
            }
            if row.action_id != candidate.action_id
                || row.family != candidate.family
                || row.target != candidate.target
                || row.memory_context_delta != Some(0.0)
                || row.decoder_family_bias.to_bits() != family.bias().to_bits()
            {
                return Err(invalid());
            }
            let expected: BTreeSet<_> = ids.iter().copied().filter(|id| matches!(
                phenotype.synapses()[*id].kind(), CompiledSynapseKind::Decoder(c) if c.family() == candidate.family
            )).collect();
            let mut observed = BTreeSet::new();
            for value in &row.contributions {
                let id = value.global_synapse_id as usize;
                let synapse = phenotype.synapses().get(id).ok_or_else(invalid)?;
                let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
                    return Err(invalid());
                };
                if !expected.contains(&id)
                    || !observed.insert(id)
                    || coordinate.input_lane() != value.input_lane
                    || coordinate.motor_index() != value.motor_index
                    || value.genetic.to_bits() != synapse.genetic_weight().to_bits()
                    || value.alpha.to_bits() != synapse.alpha().to_bits()
                    || value.lifetime != 0.0
                    || value.fast != 0.0
                    || candidate
                        .features
                        .0
                        .get(usize::from(value.input_lane))
                        .map(|f| f.to_bits())
                        != Some(value.feature.to_bits())
                {
                    return Err(invalid());
                }
                let offset = ids.binary_search(&id).map_err(|_| invalid())?;
                coefficients[offset][side * 2] = value.motor;
                coefficients[offset][side * 2 + 1] = value.feature;
            }
            if observed != expected {
                return Err(invalid());
            }
            biases[side] = row.decoder_family_bias;
        }
        Ok(Self {
            phenotype_hash: phenotype.phenotype_hash(),
            coefficients,
            biases,
        })
    }
}

/// Frozen upstream circuit, GPU pairwise logistic loss and genetic-weight updates.
/// This optimizer is absent from normal game binaries and never writes personal
/// fast or lifetime weights into the inherited foundation.
pub struct Nano512ReadoutTrainer {
    session: GpuAuthoritativeSession,
    baseline: BrainPhenotype,
    initial: FoundationWeightAsset,
    ids: Vec<usize>,
    pairs: u32,
    weights: wgpu::Buffer,
    metrics: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    evaluate: wgpu::ComputePipeline,
    update: wgpu::ComputePipeline,
}

impl Nano512ReadoutTrainer {
    pub fn new_required(
        baseline: BrainPhenotype,
        initial: FoundationWeightAsset,
        examples: &[ProductionReadoutExample],
        rate: f32,
    ) -> Result<Self, TrainingError> {
        if examples.is_empty() || !rate.is_finite() || !(0.0..=1.0).contains(&rate) || rate == 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        let (phenotype, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&initial)?;
        // Re-export also verifies the exact original graph and frozen gene bits.
        FoundationWeightAsset::from_trained_weights(
            &baseline,
            initial.weights().to_vec(),
            TrainingStageManifest::new(1, 1, 1),
        )?;
        let ids = readout_ids(&baseline);
        if examples.iter().any(|e| {
            e.phenotype_hash != phenotype.phenotype_hash() || e.coefficients.len() != ids.len()
        }) {
            return Err(invalid());
        }
        let pairs = u32::try_from(examples.len()).map_err(|_| invalid())?;
        let coefficients: Vec<[f32; 4]> = examples
            .iter()
            .flat_map(|e| e.coefficients.iter().copied())
            .collect();
        let biases: Vec<[f32; 2]> = examples.iter().map(|e| e.biases).collect();
        let genes: Vec<f32> = ids.iter().map(|id| initial.weights()[*id]).collect();
        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
        let session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
        let (device, _) = session.backend().offline_training_device_queue()?;
        let limits = device.limits();
        if coefficients.len() as u64 * 16 > u64::from(limits.max_storage_buffer_binding_size)
            || pairs.div_ceil(64) > limits.max_compute_workgroups_per_dimension
        {
            return Err(invalid());
        }
        let buffer = |label: &'static str, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let params = buffer(
            "founder-readout-parameters",
            bytemuck::cast_slice(&[pairs, ids.len() as u32, rate.to_bits(), 0]),
            wgpu::BufferUsages::UNIFORM,
        );
        let coefficients = buffer(
            "founder-readout-observations",
            bytemuck::cast_slice(&coefficients),
            wgpu::BufferUsages::STORAGE,
        );
        let biases = buffer(
            "founder-readout-biases",
            bytemuck::cast_slice(&biases),
            wgpu::BufferUsages::STORAGE,
        );
        let weights = buffer(
            "founder-readout-genes",
            bytemuck::cast_slice(&genes),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let metrics = buffer(
            "founder-readout-metrics",
            bytemuck::cast_slice(&vec![[0.0_f32; 4]; pairs as usize]),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let entries: Vec<_> = (0..5)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: binding < 3,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("founder-readout"),
            entries: &entries,
        });
        let entries: Vec<_> = [&params, &coefficients, &biases, &weights, &metrics]
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("founder-readout"),
            layout: &bindings,
            entries: &entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("founder-readout"),
            bind_group_layouts: &[Some(&bindings)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("founder-readout"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/founder_readout.wgsl").into(),
            ),
        });
        let pipeline = |entry_point| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("founder-readout"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let evaluate = pipeline("evaluate");
        let update = pipeline("update");
        Ok(Self {
            session,
            baseline,
            initial,
            ids,
            pairs,
            weights,
            metrics,
            bind_group,
            evaluate,
            update,
        })
    }

    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        count: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("founder-readout"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }

    pub fn train_steps(&mut self, steps: u32) -> Result<(), TrainingError> {
        if steps > 4096 {
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let mut encoder = device.create_command_encoder(&Default::default());
        for _ in 0..steps {
            self.dispatch(&mut encoder, &self.evaluate, self.pairs);
            self.dispatch(&mut encoder, &self.update, self.ids.len() as u32);
        }
        queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub fn loss(&self) -> Result<f32, TrainingError> {
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(&mut encoder, &self.evaluate, self.pairs);
        queue.submit(Some(encoder.finish()));
        let values = self.readback(&self.metrics, u64::from(self.pairs) * 16)?;
        Ok(values.chunks_exact(4).map(|v| v[0]).sum::<f32>() / self.pairs as f32)
    }

    pub fn export_candidate(
        &self,
        stage: TrainingStageManifest,
    ) -> Result<FoundationWeightAsset, TrainingError> {
        let values = self.readback(&self.weights, self.ids.len() as u64 * 4)?;
        let mut weights = self.initial.weights().to_vec();
        for (id, value) in self.ids.iter().zip(values) {
            weights[*id] = value;
        }
        let candidate =
            FoundationWeightAsset::from_trained_weights(&self.baseline, weights, stage)?;
        PhenotypeCompiler::compile_nano512_readout_candidate(&candidate)?;
        Ok(candidate)
    }

    fn readback(&self, source: &wgpu::Buffer, bytes: u64) -> Result<Vec<f32>, TrainingError> {
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("founder-readout-readback"),
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(source, 0, &staging, 0, bytes);
        let command = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command.map_buffer_on_submit(&staging, wgpu::MapMode::Read, 0..bytes, move |r| {
            let _ = sender.send(r);
        });
        let submission = queue.submit(Some(command));
        if device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || receiver.recv().ok().and_then(Result::ok).is_none()
        {
            return Err(TrainingError::GpuSubmission);
        }
        let mapped = staging.slice(..).get_mapped_range();
        let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        if values.iter().any(|v| !v.is_finite()) {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(values)
    }
}
