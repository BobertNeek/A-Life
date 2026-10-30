//! Isolated CPU lowering and native single-pipeline probes; production sources unchanged.
use super::*;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Sleep,
    SleepOrderedGuards,
    Eligibility,
    Encode,
    Speech,
}
impl Target {
    pub(crate) fn entry(self) -> &'static str {
        match self {
            Self::Sleep | Self::SleepOrderedGuards => "initialize_sleep_transaction",
            Self::Eligibility => "finalize_pending_eligibility",
            Self::Encode => "encode_perception",
            Self::Speech => "decode_speech_payload",
        }
    }
    fn original(self) -> (&'static str, &'static str) {
        match self {
            Self::Sleep => (CLOSED_LOOP_CONSOLIDATE_WGSL, "closed-loop-consolidate-wgsl"),
            Self::SleepOrderedGuards => (
                CLOSED_LOOP_CONSOLIDATE_WGSL,
                "probe-only-ordered-sleep-guards-wgsl",
            ),
            Self::Eligibility => (CLOSED_LOOP_ELIGIBILITY_WGSL, "closed-loop-eligibility-wgsl"),
            Self::Encode => (CLOSED_LOOP_ENCODE_WGSL, "closed-loop-encode-wgsl"),
            Self::Speech => (CLOSED_LOOP_DECODE_WGSL, "closed-loop-decode-wgsl"),
        }
    }
    pub(crate) fn source_variant(self) -> &'static str {
        if self == Self::SleepOrderedGuards {
            "test_only_ordered_final_and_guards"
        } else {
            "original"
        }
    }
    fn selected_source(self) -> Result<std::borrow::Cow<'static, str>> {
        if self == Self::SleepOrderedGuards {
            Ok(ordered_sleep_guards(self.original().0)?.into())
        } else {
            Ok(self.original().0.into())
        }
    }
    fn assembled(self) -> std::borrow::Cow<'static, str> {
        match self
            .selected_source()
            .expect("checked probe transformation")
        {
            std::borrow::Cow::Borrowed(source) => runtime_shader_source(source),
            std::borrow::Cow::Owned(source) => runtime_shader_source(&source).into_owned().into(),
        }
    }
}
// Deliberately narrow lexical transform, not a general WGSL rewriter. Refuse
// unexpected boolean nesting/comments rather than changing their precedence.
fn sleep_final_chain(source: &str) -> Result<(std::ops::Range<usize>, Vec<&str>)> {
    const NAME: &str = "fn sleep_transaction_valid(";
    if source.matches(NAME).count() != 1 {
        return Err("expected one sleep validation function".into());
    }
    let function_start = source.find(NAME).ok_or("missing function")?;
    let function_end = function_start
        + source[function_start..]
            .find("\n}\n")
            .ok_or("missing function end")?;
    let function = &source[function_start..function_end];
    let start = function_start + function.rfind("  return ").ok_or("missing final return")?;
    let end = start
        + source[start..function_end]
            .find(';')
            .ok_or("missing return terminator")?
        + 1;
    if !source[end..function_end].trim().is_empty() {
        return Err("return is not final".into());
    }
    let expression = source[start + 9..end - 1].trim();
    if expression.contains(['|', ';', '{', '}'])
        || expression.contains("//")
        || expression.contains("/*")
    {
        return Err("unsupported final-chain syntax".into());
    }
    let clauses = expression.split("&&").map(str::trim).collect::<Vec<_>>();
    if clauses.len()!=57 || clauses.first()!=Some(&"header.schema_version == GPU_SLEEP_SCHEMA_VERSION") || clauses.last()!=Some(&"consolidate_frame_span_within(header.replay_sample_offset,header.replay_sample_count)") {return Err("unexpected final predicate inventory".into());}
    for clause in &clauses {
        if clause.is_empty() {
            return Err("empty clause".into());
        }
        let mut delimiters = Vec::new();
        for c in clause.chars() {
            match c {
                '(' | '[' => delimiters.push(c),
                ')' if delimiters.pop() != Some('(') => {
                    return Err("nested AND or unbalanced clause".into());
                }
                ']' if delimiters.pop() != Some('[') => {
                    return Err("unbalanced clause".into());
                }
                _ => {}
            }
        }
        if !delimiters.is_empty() {
            return Err("nested AND or unbalanced clause".into());
        }
    }
    Ok((start..end, clauses))
}
fn ordered_sleep_guards(source: &str) -> Result<String> {
    let (range, clauses) = sleep_final_chain(source)?;
    let mut out = String::with_capacity(source.len() + clauses.len() * 40);
    out.push_str(&source[..range.start]);
    for clause in clauses {
        out.push_str("  if (!( ");
        out.push_str(clause);
        out.push_str(" )) { return false; }\n");
    }
    out.push_str("  return true;");
    out.push_str(&source[range.end..]);
    Ok(out)
}
fn max_block_depth(block: &naga::Block) -> usize {
    1 + block
        .iter()
        .map(|s| match s {
            naga::Statement::Block(b) => max_block_depth(b),
            naga::Statement::If { accept, reject, .. } => {
                max_block_depth(accept).max(max_block_depth(reject))
            }
            naga::Statement::Switch { cases, .. } => cases
                .iter()
                .map(|c| max_block_depth(&c.body))
                .max()
                .unwrap_or(0),
            naga::Statement::Loop {
                body, continuing, ..
            } => max_block_depth(body).max(max_block_depth(continuing)),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}
#[cfg(feature = "gpu-tests")]
pub(crate) fn compile_selected(
    device: &wgpu::Device,
    target: Target,
) -> std::result::Result<wgpu::ComputePipeline, GpuClosedLoopError> {
    eprintln!(
        "gpu_single_pipeline entry={} stage=bind_group_layout event=begin",
        target.entry()
    );
    let layout = crate::startup_diagnostics::scoped(
        device,
        "bind_group_layout",
        "closed-loop-neural-bind-group-layout",
        || create_neural_bind_group_layout(device),
    )?;
    eprintln!(
        "gpu_single_pipeline entry={} stage=pipeline_layout event=begin",
        target.entry()
    );
    let pipeline_layout = crate::startup_diagnostics::scoped(
        device,
        "pipeline_layout",
        "closed-loop-neural-pipeline-layout",
        || {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("closed-loop-neural-pipeline-layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            })
        },
    )?;
    eprintln!(
        "gpu_single_pipeline entry={} stage=selected_shader_module event=begin",
        target.entry()
    );
    let (_, label) = target.original();
    let source = target.selected_source().map_err(|error| {
        eprintln!("gpu_single_pipeline transform_refused={error}");
        GpuClosedLoopError::LayoutMismatch
    })?;
    eprintln!(
        "gpu_single_pipeline source_variant={} raw_source_bytes={}",
        target.source_variant(),
        source.len()
    );
    let shader = create_shader_module(device, label, &source)?;
    eprintln!(
        "gpu_single_pipeline entry={} stage=selected_shader_module event=ok",
        target.entry()
    );
    eprintln!("gpu_single_pipeline entry={} stage=compute_pipeline event=begin empty_constants=true zero_initialize_workgroup_memory=true cache=none",target.entry());
    let pipeline = create_compute_pipeline(device, &pipeline_layout, &shader, target.entry(), &[])?;
    eprintln!(
        "gpu_single_pipeline entry={} stage=compute_pipeline event=ok",
        target.entry()
    );
    Ok(pipeline)
}
#[derive(Default, serde::Serialize)]
struct Counts {
    blocks: usize,
    statements: usize,
    calls: usize,
    ifs: usize,
    loops: usize,
    returns: usize,
    barriers: usize,
}
fn walk(block: &naga::Block, c: &mut Counts, calls: &mut Vec<naga::Handle<naga::Function>>) {
    c.blocks += 1;
    for s in block.iter() {
        c.statements += 1;
        match s {
            naga::Statement::Block(b) => walk(b, c, calls),
            naga::Statement::If { accept, reject, .. } => {
                c.ifs += 1;
                walk(accept, c, calls);
                walk(reject, c, calls);
            }
            naga::Statement::Switch { cases, .. } => {
                for case in cases {
                    walk(&case.body, c, calls);
                }
            }
            naga::Statement::Loop {
                body, continuing, ..
            } => {
                c.loops += 1;
                walk(body, c, calls);
                walk(continuing, c, calls);
            }
            naga::Statement::Call { function, .. } => {
                c.calls += 1;
                calls.push(*function);
            }
            naga::Statement::Return { .. } => c.returns += 1,
            naga::Statement::ControlBarrier(..) | naga::Statement::MemoryBarrier(..) => {
                c.barriers += 1
            }
            _ => {}
        }
    }
}
fn function_stats(
    module: &naga::Module,
    f: &naga::Function,
    layouter: &naga::proc::Layouter,
) -> Value {
    let mut c = Counts::default();
    walk(&f.body, &mut c, &mut Vec::new());
    let locals=f.local_variables.iter().map(|(_,v)|json!({"name":v.name,"type":format!("{:?}",module.types[v.ty].inner),"layout_bytes":layouter[v.ty].size})).collect::<Vec<_>>();
    let arguments = f.arguments.iter().map(|a| json!({"name":a.name,"type_name":module.types[a.ty].name,"kind":match module.types[a.ty].inner {naga::TypeInner::Struct{..}=>"Struct",naga::TypeInner::Array{..}=>"Array",_=>"Other"},"layout_bytes":layouter[a.ty].size})).collect::<Vec<_>>();
    let mut aggregates = BTreeMap::new();
    for (_, e) in f.expressions.iter() {
        if let naga::Expression::Compose { ty, .. } = e {
            if matches!(
                module.types[*ty].inner,
                naga::TypeInner::Array { .. } | naga::TypeInner::Struct { .. }
            ) {
                *aggregates.entry(*ty).or_insert(0usize) += 1;
            }
        }
    }
    let aggregate_composes=aggregates.iter().map(|(ty,count)|json!({"type_name":module.types[*ty].name,"kind":if matches!(module.types[*ty].inner,naga::TypeInner::Array{..}){"Array"}else{"Struct"},"layout_bytes":layouter[*ty].size,"expression_count":count})).collect::<Vec<_>>();
    let struct_args = f
        .arguments
        .iter()
        .filter(|a| matches!(module.types[a.ty].inner, naga::TypeInner::Struct { .. }))
        .count();
    let divmod = f
        .expressions
        .iter()
        .filter(|(_, e)| {
            matches!(
                e,
                naga::Expression::Binary {
                    op: naga::BinaryOperator::Divide | naga::BinaryOperator::Modulo,
                    ..
                }
            )
        })
        .count();
    let ands = f
        .expressions
        .iter()
        .filter(|(_, e)| {
            matches!(
                e,
                naga::Expression::Binary {
                    op: naga::BinaryOperator::LogicalAnd,
                    ..
                }
            )
        })
        .count();
    json!({"name":f.name,"max_block_nesting_depth":max_block_depth(&f.body),"expressions":f.expressions.len(),"control":c,"local_variables":locals,"arguments":arguments,"aggregate_expression_composes":aggregate_composes,"struct_arguments_by_value":struct_args,"divmod_expressions":divmod,"logical_and_expressions":ands})
}
fn ir_stats(module: &naga::Module, entry: &str) -> Result<Value> {
    let ep = module
        .entry_points
        .iter()
        .find(|e| e.name == entry)
        .ok_or("missing entry")?;
    let mut layout = naga::proc::Layouter::default();
    layout.update(module.to_ctx())?;
    let mut reached = BTreeSet::new();
    let mut queue = Vec::new();
    walk(&ep.function.body, &mut Counts::default(), &mut queue);
    while let Some(h) = queue.pop() {
        if reached.insert(h) {
            walk(
                &module.functions[h].body,
                &mut Counts::default(),
                &mut queue,
            );
        }
    }
    let globals=module.global_variables.iter().map(|(_,g)|json!({"name":g.name,"address_space":format!("{:?}",g.space),"type":format!("{:?}",module.types[g.ty].inner),"layout_bytes_minimum":layout[g.ty].size})).collect::<Vec<_>>();
    let arrays=module.types.iter().filter_map(|(h,t)|if let naga::TypeInner::Array{size,stride,..}=t.inner {Some(json!({"type_handle":h.index(),"size":format!("{size:?}"),"stride":stride,"layout_bytes":layout[h].size}))}else{None}).collect::<Vec<_>>();
    let reachable = reached
        .iter()
        .map(|h| function_stats(module, &module.functions[*h], &layout))
        .collect::<Vec<_>>();
    let entry_stats = function_stats(module, &ep.function, &layout);
    let total_expressions = module
        .functions
        .iter()
        .map(|(_, f)| f.expressions.len())
        .sum::<usize>()
        + module
            .entry_points
            .iter()
            .map(|e| e.function.expressions.len())
            .sum::<usize>();
    Ok(
        json!({"module_functions":module.functions.len(),"module_entry_points":module.entry_points.len(),"module_types":module.types.len(),"module_total_function_expressions":total_expressions,"reachable_regular_functions":reached.len(),"entry":entry_stats,"reachable_functions":reachable,"global_variables":globals,"array_types_in_module":arrays}),
    )
}
fn spirv_counts(words: &[u32]) -> Result<Value> {
    if words.len() < 5 {
        return Err("short SPIR-V".into());
    }
    let mut cursor = 5;
    let mut opcodes = BTreeMap::new();
    let mut instructions = 0;
    while cursor < words.len() {
        let n = (words[cursor] >> 16) as usize;
        if n == 0 || cursor + n > words.len() {
            return Err("bad SPIR-V instruction range".into());
        }
        *opcodes.entry(words[cursor] & 65535).or_insert(0usize) += 1;
        instructions += 1;
        cursor += n;
    }
    Ok(
        json!({"words":words.len(),"bytes":words.len()*4,"instructions":instructions,"opcode_counts":opcodes}),
    )
}
fn emit(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    entry: &str,
    dir: &Path,
    phase: &str,
) -> Result<Value> {
    let mut spv = naga::back::spv::Options {
        fake_missing_bindings: false,
        ..Default::default()
    };
    let mut hlsl = naga::back::hlsl::Options {
        fake_missing_bindings: false,
        ..Default::default()
    };
    for (_, g) in module.global_variables.iter() {
        if let Some(b) = g.binding {
            spv.binding_map.insert(
                b,
                naga::back::spv::BindingInfo {
                    descriptor_set: b.group,
                    binding: b.binding,
                    binding_array_size: None,
                },
            );
            hlsl.binding_map.insert(
                b,
                naga::back::hlsl::BindTarget {
                    space: b.group as u8,
                    register: b.binding,
                    ..Default::default()
                },
            );
        }
    }
    let words = naga::back::spv::write_vec(
        module,
        info,
        &spv,
        Some(&naga::back::spv::PipelineOptions {
            shader_stage: naga::ShaderStage::Compute,
            entry_point: entry.into(),
        }),
    )?;
    fs::write(
        dir.join(format!("{entry}-{phase}.spv")),
        words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<_>>(),
    )?;
    let mut text = String::new();
    let opts = naga::back::hlsl::PipelineOptions {
        entry_point: Some((naga::ShaderStage::Compute, entry.into())),
    };
    let reflected =
        naga::back::hlsl::Writer::new(&mut text, &hlsl, &opts).write(module, info, None)?;
    if reflected.entry_point_names.iter().any(|n| n.is_err()) {
        return Err(format!("HLSL reflection failed: {:?}", reflected.entry_point_names).into());
    }
    fs::write(dir.join(format!("{entry}-{phase}.hlsl")), &text)?;
    Ok(
        json!({"spirv":spirv_counts(&words)?,"hlsl_bytes":text.len(),"hlsl_lines":text.lines().count(),"hlsl_reflected_entry_names":format!("{:?}",reflected.entry_point_names),"spirv_options":format!("{spv:?}"),"hlsl_options":format!("{hlsl:?}"),"emission_scope":"CPU Naga options recorded; not actual adapter-specialized HAL options or native driver compilation"}),
    )
}
#[test]
fn original_target_sources_and_seven_bindings_are_preserved() {
    let mut names = BTreeSet::new();
    for target in [
        Target::Sleep,
        Target::Eligibility,
        Target::Encode,
        Target::Speech,
    ] {
        assert!(names.insert(target.entry()));
        let source = target.assembled();
        assert!(source.contains(&format!("fn {}(", target.entry())));
        validate_production_shader_contract(&source, &[target.entry()]).unwrap();
    }
    assert_eq!(
        crate::GpuClassBucketBuffers::neural_binding_manifest().len(),
        7
    );
}
#[test]
#[cfg(feature = "training-rollout")]
fn original_selected_entry_naga_lowering_cpu_only() -> Result<()> {
    let dir = std::env::var_os("ALIFE_SINGLE_PIPELINE_OUTPUT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "alife-single-pipeline-lowering-{}",
                std::process::id()
            ))
        });
    fs::create_dir_all(&dir)?;
    let mut rows = Vec::new();
    for target in [
        Target::Sleep,
        Target::Eligibility,
        Target::Encode,
        Target::Speech,
    ] {
        let source = target.assembled();
        fs::write(
            dir.join(format!("{}.wgsl", target.entry())),
            source.as_bytes(),
        )?;
        let module = naga::front::wgsl::parse_str(&source)?;
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)?;
        let (processed, processed_info) = naga::back::pipeline_constants::process_overrides(
            &module,
            &info,
            Some((naga::ShaderStage::Compute, target.entry())),
            &Default::default(),
        )?;
        assert_eq!(processed.entry_points.len(), 1);
        rows.push(json!({"entry":target.entry(),"assembled_source_bytes":source.len(),"assembled_ir":ir_stats(&module,target.entry())?,"processed_ir":ir_stats(&processed,target.entry())?,"raw_emission":emit(&module,&info,target.entry(),&dir,"raw")?,"processed_emission":emit(&processed,&processed_info,target.entry(),&dir,"processed")?,"preprocess":"actual Naga29 process_overrides selected compute entry and empty constants, as HAL; borrowed shortcut retained if applicable"}));
        fs::write(
            dir.join("lowering-report.json"),
            serde_json::to_vec_pretty(&rows)?,
        )?;
    }
    eprintln!("CPU-only Naga lowering retained at {}", dir.display());
    Ok(())
}

#[test]
fn ordered_guards_preserve_source_predicates_and_short_circuit_trace() -> Result<()> {
    let original = CLOSED_LOOP_CONSOLIDATE_WGSL;
    let (range, clauses) = sleep_final_chain(original)?;
    let variant = ordered_sleep_guards(original)?;
    let changed_len = variant.len() - (original.len() - (range.end - range.start));
    assert_eq!(&variant[..range.start], &original[..range.start]);
    assert_eq!(
        &variant[range.start + changed_len..],
        &original[range.end..]
    );
    let changed = &variant[range.start..range.start + changed_len];
    let recovered = changed
        .lines()
        .take(57)
        .map(|line| {
            line.strip_prefix("  if (!( ")
                .and_then(|s| s.strip_suffix(" )) { return false; }"))
                .expect("exact guard shape")
        })
        .collect::<Vec<_>>();
    assert_eq!(recovered, clauses);
    assert_eq!(changed.lines().last(), Some("  return true;"));
    // Actual extracted predicate strings are trace labels, not invented numeric
    // implementations. Enumerate every first-false position plus all true.
    for first_false in 0..=clauses.len() {
        let mut original_trace = Vec::new();
        let original_result = clauses.iter().enumerate().all(|(i, c)| {
            original_trace.push(*c);
            i != first_false
        });
        let mut variant_trace = Vec::new();
        let mut variant_result = true;
        for (i, c) in recovered.iter().enumerate() {
            variant_trace.push(*c);
            if i == first_false {
                variant_result = false;
                break;
            }
        }
        assert_eq!(variant_result, original_result);
        assert_eq!(variant_trace, original_trace);
        assert_eq!(
            original_trace,
            clauses[..(first_false + 1).min(clauses.len())]
        );
    }
    assert!(ordered_sleep_guards(&format!("{original}\n{original}")).is_err());
    assert!(ordered_sleep_guards(&original.replace(
        "header.flags == 0u && header.reserved == 0u",
        "header.flags == 0u || header.reserved == 0u"
    ))
    .is_err());
    assert!(ordered_sleep_guards(&original.replace(
        "header.flags == 0u && header.reserved == 0u",
        "(header.flags == 0u && header.reserved == 0u)"
    ))
    .is_err());
    Ok(())
}
#[test]
#[cfg(feature = "training-rollout")]
fn ordered_guards_original_and_variant_cpu_lowering() -> Result<()> {
    let base = std::env::var_os("ALIFE_SINGLE_PIPELINE_OUTPUT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("alife-ordered-sleep-{}", std::process::id()))
        });
    let dir = base.join("ordered-guards");
    fs::create_dir_all(&dir)?;
    let mut rows = Vec::new();
    for (target, phase) in [
        (Target::Sleep, "original"),
        (Target::SleepOrderedGuards, "variant"),
    ] {
        let source = target.assembled();
        fs::write(dir.join(format!("{phase}.wgsl")), source.as_bytes())?;
        validate_production_shader_contract(&source, &[target.entry()])?;
        let module = naga::front::wgsl::parse_str(&source)?;
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)?;
        let (processed, pi) = naga::back::pipeline_constants::process_overrides(
            &module,
            &info,
            Some((naga::ShaderStage::Compute, target.entry())),
            &Default::default(),
        )?;
        rows.push(json!({"variant":phase,"source_bytes":source.len(),"assembled_ir":ir_stats(&module,target.entry())?,"processed_ir":ir_stats(&processed,target.entry())?,"emission":emit(&processed,&pi,target.entry(),&dir,phase)?}));
    }
    let entry = Target::Sleep.entry();
    let equal = |extension: &str| -> Result<bool> {
        Ok(fs::read(dir.join(format!("{entry}-original.{extension}")))?
            == fs::read(dir.join(format!("{entry}-variant.{extension}")))?)
    };
    fs::write(
        dir.join("comparison.json"),
        serde_json::to_vec_pretty(
            &json!({"rows":rows,"processed_spirv_bytes_equal":equal("spv")?,"processed_hlsl_bytes_equal":equal("hlsl")?,"scope":"CPU Naga options, not native driver evidence; no numeric or code-size pass threshold"}),
        )?,
    )?;
    Ok(())
}
