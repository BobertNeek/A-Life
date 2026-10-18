//! Full sampled residual updates on frozen production GPU features. The caller
//! owns campaign/PPO accounting; this kernel never updates the actor or world.
use super::{invalid, read_gpu_f32, PpoValueHeadCheckpoint};
use crate::{AdamWConfig, TrainingError};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use wgpu::util::DeviceExt;

pub const CRITIC_RESIDUAL_WGSL: &str = include_str!("../../shaders/critic_residual.wgsl");

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CriticResidualRow {
    pub current: u32,
    pub next: u32,
    pub reward: f32,
    /// Actual elapsed-time discount; zero only for an actual terminal.
    pub discount: f32,
}

pub struct FrozenCriticResidualExperiment {
    head: wgpu::Buffer,
    header: wgpu::Buffer,
    output: wgpu::Buffer,
    bind: wgpu::BindGroup,
    predict: wgpu::ComputePipeline,
    gradient: wgpu::ComputePipeline,
    update: wgpu::ComputePipeline,
    feature_count: u32,
    row_count: u32,
    fit_rows: u32,
    step: u32,
    last_policy: Option<u64>,
}

impl FrozenCriticResidualExperiment {
    pub fn new(
        session: &GpuAuthoritativeSession,
        checkpoint: &PpoValueHeadCheckpoint,
        features: &[Vec<f32>],
        rows: &[CriticResidualRow],
        fit_rows: usize,
    ) -> Result<Self, TrainingError> {
        checkpoint.validate()?;
        if session.authority().consumer() != GpuSessionConsumerKind::Training
            || rows.is_empty()
            || rows.len() > 4096
            || fit_rows == 0
            || fit_rows > rows.len()
            || features.is_empty()
            || features.len() > 8192
            || features.iter().any(|r| {
                r.len() != checkpoint.feature_count as usize || r.iter().any(|v| !v.is_finite())
            })
            || rows.iter().any(|r| {
                r.current as usize >= features.len()
                    || r.next as usize >= features.len()
                    || !r.reward.is_finite()
                    || !r.discount.is_finite()
                    || !(0.0..=1.0).contains(&r.discount)
            })
        {
            return Err(invalid());
        }
        let (device, _) = session.backend().offline_training_device_queue()?;
        let storage = wgpu::BufferUsages::STORAGE;
        let make = |label, bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage,
            })
        };
        let features = make(
            "critic-fixed-GPU-feature-corpus",
            bytemuck::cast_slice(&features.concat()),
            storage,
        );
        let row_words = rows
            .iter()
            .flat_map(|r| {
                [
                    r.current * checkpoint.feature_count,
                    r.next * checkpoint.feature_count,
                    r.reward.to_bits(),
                    r.discount.to_bits(),
                ]
            })
            .collect::<Vec<_>>();
        let rows_buffer = make(
            "critic-real-transition-rows",
            bytemuck::cast_slice(&row_words),
            storage,
        );
        let mut head_words = checkpoint.parameters.clone();
        head_words.resize((checkpoint.feature_count as usize + 1) * 4 + 1, 0.0);
        let head = make(
            "critic-existing-weights-Adam-and-gradient",
            bytemuck::cast_slice(&head_words),
            storage | wgpu::BufferUsages::COPY_SRC,
        );
        let header = make(
            "critic-residual-header",
            &[0; 64],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let output = make(
            "critic-residual-values",
            &vec![0; rows.len() * 16],
            storage | wgpu::BufferUsages::COPY_SRC,
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("critic-residual-layout"),
            entries: &(0..5)
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
                .collect::<Vec<_>>(),
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("critic-residual-bind"),
            layout: &layout,
            entries: &[&header, &features, &rows_buffer, &head, &output]
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: b.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("critic-residual-pipelines"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("critic-full-Bellman-residual"),
            source: wgpu::ShaderSource::Wgsl(CRITIC_RESIDUAL_WGSL.into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Ok(Self {
            predict: pipeline("predict"),
            gradient: pipeline("gradient"),
            update: pipeline("update"),
            head,
            header,
            output,
            bind,
            feature_count: checkpoint.feature_count,
            row_count: rows.len() as u32,
            fit_rows: fit_rows as u32,
            step: checkpoint.optimizer_step,
            last_policy: checkpoint.last_updated_policy_version,
        })
    }

    fn upload(
        &self,
        session: &GpuAuthoritativeSession,
        config: AdamWConfig,
        step: u32,
        coefficient: f32,
    ) -> Result<(), TrainingError> {
        config.validate()?;
        let mut words = [0u32; 16];
        words[..4].copy_from_slice(&[self.feature_count, self.row_count, self.fit_rows, step]);
        for (i, value) in [
            config.learning_rate,
            config.beta1,
            config.beta2,
            config.epsilon,
            config.weight_decay,
            config.gradient_clip,
        ]
        .iter()
        .enumerate()
        {
            words[4 + i] = value.to_bits();
        }
        if !coefficient.is_finite() || coefficient <= 0.0 {
            return Err(invalid());
        }
        words[10] = coefficient.to_bits();
        session
            .backend()
            .offline_training_device_queue()?
            .1
            .write_buffer(&self.header, 0, bytemuck::cast_slice(&words));
        Ok(())
    }

    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        groups: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("critic-only-residual"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.dispatch_workgroups(groups, 1, 1);
    }

    /// One completed GPU AdamW update advances only the critic age.
    pub fn step(
        &mut self,
        session: &GpuAuthoritativeSession,
        config: AdamWConfig,
    ) -> Result<(), TrainingError> {
        self.step_with_coefficient(session, config, 1.0)
    }

    /// The coefficient belongs to the loss, before clipping and AdamW.
    pub fn step_with_coefficient(
        &mut self,
        session: &GpuAuthoritativeSession,
        config: AdamWConfig,
        coefficient: f32,
    ) -> Result<(), TrainingError> {
        let next = self.step.checked_add(1).ok_or_else(invalid)?;
        self.upload(session, config, next, coefficient)?;
        let (device, queue) = session.backend().offline_training_device_queue()?;
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(&mut encoder, &self.predict, self.row_count.div_ceil(64));
        self.dispatch(
            &mut encoder,
            &self.gradient,
            (self.feature_count + 1).div_ceil(64),
        );
        self.dispatch(&mut encoder, &self.update, 1);
        queue.submit(Some(encoder.finish()));
        let flag = u64::from(self.feature_count + 1) * 16;
        if read_gpu_f32(session, &self.head, flag..flag + 4)?.as_slice() != [1.0] {
            return Err(TrainingError::MalformedReadback);
        }
        self.step = next;
        Ok(())
    }

    pub fn evaluate(
        &self,
        session: &GpuAuthoritativeSession,
    ) -> Result<Vec<[f32; 4]>, TrainingError> {
        self.upload(session, AdamWConfig::default(), self.step.max(1), 1.0)?;
        let (device, queue) = session.backend().offline_training_device_queue()?;
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(&mut encoder, &self.predict, self.row_count.div_ceil(64));
        queue.submit(Some(encoder.finish()));
        let words = read_gpu_f32(session, &self.output, 0..u64::from(self.row_count) * 16)?;
        if words.iter().any(|v| !v.is_finite()) {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(words.as_chunks::<4>().0.to_vec())
    }

    pub fn gradient_snapshot(
        &self,
        session: &GpuAuthoritativeSession,
    ) -> Result<Vec<f32>, TrainingError> {
        self.upload(session, AdamWConfig::default(), self.step.max(1), 1.0)?;
        let (device, queue) = session.backend().offline_training_device_queue()?;
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(&mut encoder, &self.predict, self.row_count.div_ceil(64));
        self.dispatch(
            &mut encoder,
            &self.gradient,
            (self.feature_count + 1).div_ceil(64),
        );
        queue.submit(Some(encoder.finish()));
        let start = u64::from(self.feature_count + 1) * 12;
        read_gpu_f32(
            session,
            &self.head,
            start..start + u64::from(self.feature_count + 1) * 4,
        )
    }

    pub fn checkpoint(
        &self,
        session: &GpuAuthoritativeSession,
    ) -> Result<PpoValueHeadCheckpoint, TrainingError> {
        let result = PpoValueHeadCheckpoint {
            feature_count: self.feature_count,
            optimizer_step: self.step,
            last_updated_policy_version: self.last_policy,
            parameters: read_gpu_f32(
                session,
                &self.head,
                0..u64::from(self.feature_count + 1) * 12,
            )?,
        };
        result.validate()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn residual_shader_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(super::CRITIC_RESIDUAL_WGSL).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
